use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct VaultClient {
    addr: String,
    role_id: String,
    secret_id: String,
    namespace: Option<String>,
    kv_mount: String,
    http: reqwest::Client,
    token: Arc<RwLock<Option<String>>>,
}

#[derive(Debug)]
pub struct VaultError {
    message: String,
}

impl VaultError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for VaultError {}

impl VaultClient {
    pub fn new(
        addr: String,
        role_id: String,
        secret_id: String,
        namespace: Option<String>,
        kv_mount: String,
    ) -> Self {
        Self {
            addr,
            role_id,
            secret_id,
            namespace,
            kv_mount,
            http: reqwest::Client::new(),
            token: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn write_db_connection_string(
        &self,
        secret_path: &str,
        connection_string: &str,
    ) -> Result<(), VaultError> {
        let url = format!("{}/v1/{}/data/{}", self.addr, self.kv_mount, secret_path);
        let payload = KvWriteRequest {
            data: DbSecret {
                connection_string: connection_string.to_string(),
            },
        };

        let url_first = url.clone();
        let payload_first = payload.clone();

        let result = self.with_token(|token, namespace, http| async move {
            let mut req = http
                .post(url_first)
                .bearer_auth(token)
                .json(&payload_first);
            if let Some(ns) = namespace {
                req = req.header("X-Vault-Namespace", ns);
            }
            let resp = req.send().await.map_err(|e| VaultError::new(e.to_string()))?;

            if resp.status().is_success() {
                return Ok(());
            }

            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(VaultError::new(format!(
                "Vault write failed: {} - {}",
                status, body
            )))
        })
        .await;

        if let Err(err) = &result && is_auth_error(err) {
            self.clear_token().await;
            return self
                .with_token(|token, namespace, http| async move {
                    let mut req = http.post(url).bearer_auth(token).json(&payload);
                    if let Some(ns) = namespace {
                        req = req.header("X-Vault-Namespace", ns);
                    }
                    let resp = req.send().await.map_err(|e| VaultError::new(e.to_string()))?;

                    if resp.status().is_success() {
                        return Ok(());
                    }

                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    Err(VaultError::new(format!(
                        "Vault write failed: {} - {}",
                        status, body
                    )))
                })
                .await;
        }

        result
    }

    pub async fn read_db_connection_string(
        &self,
        secret_path: &str,
    ) -> Result<String, VaultError> {
        let url = format!("{}/v1/{}/data/{}", self.addr, self.kv_mount, secret_path);
        let url_first = url.clone();

        let result = self.with_token(|token, namespace, http| async move {
            let mut req = http.get(url_first).bearer_auth(token);
            if let Some(ns) = namespace {
                req = req.header("X-Vault-Namespace", ns);
            }
            let resp = req.send().await.map_err(|e| VaultError::new(e.to_string()))?;

            if resp.status().is_success() {
                let body: KvReadResponse = resp
                    .json()
                    .await
                    .map_err(|e| VaultError::new(e.to_string()))?;
                return Ok(body.data.data.connection_string);
            }

            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(VaultError::new(format!(
                "Vault read failed: {} - {}",
                status, body
            )))
        })
        .await;

        if let Err(err) = &result && is_auth_error(err) {
            self.clear_token().await;
            return self
                .with_token(|token, namespace, http| async move {
                    let mut req = http.get(url).bearer_auth(token);
                    if let Some(ns) = namespace {
                        req = req.header("X-Vault-Namespace", ns);
                    }
                    let resp =
                        req.send().await.map_err(|e| VaultError::new(e.to_string()))?;

                    if resp.status().is_success() {
                        let body: KvReadResponse = resp
                            .json()
                            .await
                            .map_err(|e| VaultError::new(e.to_string()))?;
                        return Ok(body.data.data.connection_string);
                    }

                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    Err(VaultError::new(format!(
                        "Vault read failed: {} - {}",
                        status, body
                    )))
                })
                .await;
        }

        result
    }

    pub async fn delete_secret_path(&self, secret_path: &str) -> Result<(), VaultError> {
        let url = format!("{}/v1/{}/metadata/{}", self.addr, self.kv_mount, secret_path);

        self.with_token(|token, namespace, http| async move {
            let mut req = http.delete(url).bearer_auth(token);
            if let Some(ns) = namespace {
                req = req.header("X-Vault-Namespace", ns);
            }
            let resp = req.send().await.map_err(|e| VaultError::new(e.to_string()))?;

            if resp.status().is_success() {
                return Ok(());
            }

            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(VaultError::new(format!(
                "Vault delete failed: {} - {}",
                status, body
            )))
        })
        .await
    }

    async fn with_token<F, Fut, T>(&self, f: F) -> Result<T, VaultError>
    where
        F: FnOnce(String, Option<String>, reqwest::Client) -> Fut,
        Fut: std::future::Future<Output = Result<T, VaultError>>,
    {
        let token = self.get_or_login_token().await?;
        let namespace = self.namespace.clone();
        let http = self.http.clone();

        f(token, namespace, http).await
    }

    async fn get_or_login_token(&self) -> Result<String, VaultError> {
        {
            let read = self.token.read().await;
            if let Some(token) = read.clone() {
                return Ok(token);
            }
        }

        let token = self.login_approle().await?;
        let mut write = self.token.write().await;
        *write = Some(token.clone());
        Ok(token)
    }

    async fn clear_token(&self) {
        let mut write = self.token.write().await;
        *write = None;
    }

    async fn login_approle(&self) -> Result<String, VaultError> {
        let url = format!("{}/v1/auth/approle/login", self.addr);
        let payload = AppRoleLoginRequest {
            role_id: self.role_id.clone(),
            secret_id: self.secret_id.clone(),
        };

        let mut req = self.http.post(url).json(&payload);
        if let Some(ns) = &self.namespace {
            req = req.header("X-Vault-Namespace", ns);
        }
        let resp = req.send().await.map_err(|e| VaultError::new(e.to_string()))?;

        if resp.status() != StatusCode::OK {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(VaultError::new(format!(
                "Vault AppRole login failed: {} - {}",
                status, body
            )));
        }

        let body: AppRoleLoginResponse = resp
            .json()
            .await
            .map_err(|e| VaultError::new(e.to_string()))?;

        Ok(body.auth.client_token)
    }
}

#[derive(Debug, Serialize)]
struct AppRoleLoginRequest {
    role_id: String,
    secret_id: String,
}

#[derive(Debug, Deserialize)]
struct AppRoleLoginResponse {
    auth: AppRoleAuth,
}

#[derive(Debug, Deserialize)]
struct AppRoleAuth {
    client_token: String,
}

#[derive(Debug, Serialize, Clone)]
struct KvWriteRequest {
    data: DbSecret,
}

#[derive(Debug, Deserialize)]
struct KvReadResponse {
    data: KvReadData,
}

#[derive(Debug, Deserialize)]
struct KvReadData {
    data: DbSecret,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DbSecret {
    connection_string: String,
}

fn is_auth_error(err: &VaultError) -> bool {
    err.message.contains("403") || err.message.contains("permission denied")
}
