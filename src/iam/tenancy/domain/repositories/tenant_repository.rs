use crate::iam::tenancy::domain::model::{
    aggregates::tenant::Tenant,
    value_objects::{tenant_anon_key::TenantAnonKey, tenant_id::TenantId},
};
use std::error::Error;
use std::future::Future;

#[cfg_attr(test, mockall::automock)]
pub trait TenantRepository: Send + Sync {
    fn save(
        &self,
        tenant: Tenant,
    ) -> impl Future<Output = Result<Tenant, Box<dyn Error + Send + Sync>>> + Send;

    fn find_by_anon_key(
        &self,
        anon_key: &TenantAnonKey,
    ) -> impl Future<Output = Result<Option<Tenant>, Box<dyn Error + Send + Sync>>> + Send;

    fn find_by_id(
        &self,
        tenant_id: TenantId,
    ) -> impl Future<Output = Result<Option<Tenant>, Box<dyn Error + Send + Sync>>> + Send;

    fn delete_by_id(
        &self,
        tenant_id: TenantId,
    ) -> impl Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send;

    fn rotate_keys(
        &self,
        tenant_id: TenantId,
        anon_key: TenantAnonKey,
        secret_key_hash: String,
    ) -> impl Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send;
}
