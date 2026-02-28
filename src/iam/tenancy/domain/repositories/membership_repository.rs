use crate::iam::tenancy::domain::model::{
    aggregates::membership::Membership, value_objects::tenant_id::TenantId,
};
use std::error::Error;
use std::future::Future;
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
pub trait MembershipRepository: Send + Sync {
    fn find_by_user_and_tenant(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> impl Future<Output = Result<Option<Membership>, Box<dyn Error + Send + Sync>>> + Send;
}
