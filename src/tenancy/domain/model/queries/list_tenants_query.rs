use crate::tenancy::domain::error::TenantError;

#[derive(Debug)]
pub struct ListTenantsQuery {
    pub offset: u64,
    pub limit: u64,
}

impl ListTenantsQuery {
    pub fn new(offset: Option<u64>, limit: Option<u64>) -> Result<Self, TenantError> {
        Ok(Self {
            offset: offset.unwrap_or(0),
            limit: limit.unwrap_or(20).min(100),
        })
    }
}
