use rand::distr::Alphanumeric;
use rand::Rng;
use std::process::Command;

#[derive(Clone)]
pub struct DockerProvisioner {
    image: String,
    network: String,
    host: String,
    db_user: String,
    db_name_prefix: String,
    memory_mb: Option<i64>,
    cpu_cores: Option<f64>,
}

#[derive(Debug)]
pub struct DockerProvisionError {
    message: String,
}

impl DockerProvisionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DockerProvisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DockerProvisionError {}

pub struct ProvisionedDb {
    pub connection_string: String,
    pub container_id: String,
}

impl DockerProvisioner {
    pub fn new(
        image: String,
        network: String,
        host: String,
        db_user: String,
        db_name_prefix: String,
        memory_mb: Option<i64>,
        cpu_cores: Option<f64>,
    ) -> Result<Self, DockerProvisionError> {
        Ok(Self {
            image,
            network,
            host,
            db_user,
            db_name_prefix,
            memory_mb,
            cpu_cores,
        })
    }

    pub async fn create_tenant_db(
        &self,
        tenant_name: &str,
    ) -> Result<ProvisionedDb, DockerProvisionError> {
        self.ensure_network().await?;
        self.pull_image().await?;

        let safe_name = tenant_name.replace('-', "_");
        let db_name = format!("{}{}", self.db_name_prefix, safe_name);
        let container_name = format!("tenant-db-{}", safe_name);

        let password: String = {
            let mut rng = rand::rng();
            (0..32).map(|_| rng.sample(Alphanumeric) as char).collect()
        };

        let mut args: Vec<String> = vec![
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(),
            container_name,
            "--network".to_string(),
            self.network.clone(),
            "-e".to_string(),
            format!("POSTGRES_USER={}", self.db_user),
            "-e".to_string(),
            format!("POSTGRES_PASSWORD={}", password),
            "-e".to_string(),
            format!("POSTGRES_DB={}", db_name),
            "-p".to_string(),
            "0:5432".to_string(),
        ];

        if let Some(memory_mb) = self.memory_mb {
            let memory_arg = format!("{}m", memory_mb);
            args.push("--memory".to_string());
            args.push(memory_arg);
        }

        if let Some(cpu_cores) = self.cpu_cores {
            let cpu_arg = cpu_cores.to_string();
            args.push("--cpus".to_string());
            args.push(cpu_arg);
        }

        args.push(self.image.clone());

        let container_id = run_docker(&args).await?;
        let port_mapping = run_docker(&[
            "port".to_string(),
            container_id.trim().to_string(),
            "5432/tcp".to_string(),
        ])
        .await?;

        let port = parse_port(&port_mapping).ok_or_else(|| {
            DockerProvisionError::new("Failed to parse tenant DB port mapping")
        })?;

        let connection_string = format!(
            "postgres://{}:{}@{}:{}/{}",
            self.db_user, password, self.host, port, db_name
        );

        Ok(ProvisionedDb {
            connection_string,
            container_id: container_id.trim().to_string(),
        })
    }

    pub async fn remove_container(&self, container_id: &str) -> Result<(), DockerProvisionError> {
        let _ = run_docker(&[
            "rm".to_string(),
            "-f".to_string(),
            container_id.to_string(),
        ])
        .await;
        Ok(())
    }

    async fn ensure_network(&self) -> Result<(), DockerProvisionError> {
        if run_docker(&[
            "network".to_string(),
            "inspect".to_string(),
            self.network.clone(),
        ])
        .await
        .is_ok()
        {
            return Ok(());
        }
        let _ = run_docker(&[
            "network".to_string(),
            "create".to_string(),
            self.network.clone(),
        ])
        .await?;
        Ok(())
    }

    async fn pull_image(&self) -> Result<(), DockerProvisionError> {
        let _ = run_docker(&["pull".to_string(), self.image.clone()]).await?;
        Ok(())
    }
}

async fn run_docker(args: &[String]) -> Result<String, DockerProvisionError> {
    let args_vec = args.to_vec();
    let output = tokio::task::spawn_blocking(move || {
        Command::new("docker").args(&args_vec).output()
    })
    .await
    .map_err(|e| DockerProvisionError::new(e.to_string()))?
    .map_err(|e| DockerProvisionError::new(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DockerProvisionError::new(format!(
            "docker {:?} failed: {}",
            args, stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_port(mapping: &str) -> Option<String> {
    let parts: Vec<&str> = mapping.trim().split(':').collect();
    parts.last().map(|v| v.trim().to_string())
}
