use crate::iam::tenancy::domain::model::value_objects::{
    membership_id::MembershipId, membership_role::MembershipRole,
    membership_status::MembershipStatus, tenant_id::TenantId,
};
use crate::shared::domain::model::entities::auditable_model::AuditableModel;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Membership {
    id: MembershipId,
    tenant_id: TenantId,
    user_id: Uuid,
    role: MembershipRole,
    status: MembershipStatus,
    audit: AuditableModel,
}

impl Membership {
    pub fn new(
        id: MembershipId,
        tenant_id: TenantId,
        user_id: Uuid,
        role: MembershipRole,
        status: MembershipStatus,
        audit: AuditableModel,
    ) -> Self {
        Self {
            id,
            tenant_id,
            user_id,
            role,
            status,
            audit,
        }
    }

    pub fn id(&self) -> MembershipId {
        self.id
    }

    pub fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    pub fn role(&self) -> &MembershipRole {
        &self.role
    }

    pub fn status(&self) -> &MembershipStatus {
        &self.status
    }

    pub fn audit(&self) -> &AuditableModel {
        &self.audit
    }
}
