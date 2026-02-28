use crate::iam::tenancy::domain::model::value_objects::{
    google_oauth_tenant_configuration::GoogleOAuthTenantConfiguration,
    tenant_anon_key::TenantAnonKey, tenant_id::TenantId, tenant_name::TenantName,
    tenant_schema_name::TenantSchemaName, tenant_status::TenantStatus,
};
use crate::shared::domain::model::entities::auditable_model::AuditableModel;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Tenant {
    id: TenantId,
    name: TenantName,
    schema_name: TenantSchemaName,
    admin_user_id: Uuid,
    anon_key: TenantAnonKey,
    secret_key_hash: String,
    google_oauth_configuration: Option<GoogleOAuthTenantConfiguration>,
    status: TenantStatus,
    audit: AuditableModel,
}

impl Tenant {
    pub fn new(
        id: TenantId,
        name: TenantName,
        schema_name: TenantSchemaName,
        admin_user_id: Uuid,
        anon_key: TenantAnonKey,
        secret_key_hash: String,
        google_oauth_configuration: Option<GoogleOAuthTenantConfiguration>,
        status: TenantStatus,
        audit: AuditableModel,
    ) -> Self {
        Self {
            id,
            name,
            schema_name,
            admin_user_id,
            anon_key,
            secret_key_hash,
            google_oauth_configuration,
            status,
            audit,
        }
    }

    pub fn id(&self) -> TenantId {
        self.id
    }

    pub fn name(&self) -> &TenantName {
        &self.name
    }

    pub fn schema_name(&self) -> &TenantSchemaName {
        &self.schema_name
    }

    pub fn admin_user_id(&self) -> Uuid {
        self.admin_user_id
    }

    pub fn anon_key(&self) -> &TenantAnonKey {
        &self.anon_key
    }

    pub fn secret_key_hash(&self) -> &str {
        &self.secret_key_hash
    }

    pub fn google_oauth_configuration(&self) -> Option<&GoogleOAuthTenantConfiguration> {
        self.google_oauth_configuration.as_ref()
    }

    pub fn status(&self) -> &TenantStatus {
        &self.status
    }

    pub fn audit(&self) -> &AuditableModel {
        &self.audit
    }
}
