use crate::iam::tenancy::domain::{
    model::{
        aggregates::membership::Membership as DomainMembership,
        value_objects::{
            membership_id::MembershipId, membership_role::MembershipRole,
            membership_status::MembershipStatus, tenant_id::TenantId,
        },
    },
    repositories::membership_repository::MembershipRepository,
};
use crate::iam::tenancy::infrastructure::persistence::postgres::membership_model::{
    Column, Entity as MembershipEntity,
};
use crate::shared::domain::model::entities::auditable_model::AuditableModel;
use sea_orm::*;
use std::error::Error;
use std::str::FromStr;
use uuid::Uuid;

pub struct MembershipRepositoryImpl {
    db: DatabaseConnection,
}

impl MembershipRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

impl MembershipRepository for MembershipRepositoryImpl {
    async fn find_by_user_and_tenant(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<Option<DomainMembership>, Box<dyn Error + Send + Sync>> {
        let model = MembershipEntity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::TenantId.eq(tenant_id.value()))
            .one(&self.db)
            .await?;

        match model {
            Some(m) => {
                let membership = DomainMembership::new(
                    MembershipId::from_uuid(m.id).map_err(Box::<dyn Error + Send + Sync>::from)?,
                    TenantId::from_uuid(m.tenant_id)
                        .map_err(Box::<dyn Error + Send + Sync>::from)?,
                    m.user_id,
                    MembershipRole::from_str(&m.role)
                        .map_err(Box::<dyn Error + Send + Sync>::from)?,
                    MembershipStatus::from_str(&m.status)
                        .map_err(Box::<dyn Error + Send + Sync>::from)?,
                    AuditableModel {
                        created_at: m.created_at.into(),
                        updated_at: m.updated_at.into(),
                    },
                );
                Ok(Some(membership))
            }
            None => Ok(None),
        }
    }
}
