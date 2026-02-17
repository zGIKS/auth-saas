#[derive(Debug, Clone)]
pub struct CountAdminAccountsQuery;

impl CountAdminAccountsQuery {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CountAdminAccountsQuery {
    fn default() -> Self {
        Self::new()
    }
}
