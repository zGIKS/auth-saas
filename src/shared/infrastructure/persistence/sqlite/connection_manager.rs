use sea_orm::{Database, DatabaseConnection, DbErr};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct ConnectionManager {
    data_dir: String,
    main_db_url: String,
}

impl ConnectionManager {
    pub fn new(main_db_url: String, data_dir: String) -> Self {
        Self {
            main_db_url,
            data_dir,
        }
    }

    pub async fn get_main_connection(&self) -> Result<DatabaseConnection, DbErr> {
        Database::connect(&self.main_db_url).await
    }

    pub async fn get_tenant_connection(&self, schema_name: &str) -> Result<DatabaseConnection, DbErr> {
        let db_path = Path::new(&self.data_dir).join(format!("{}.db", schema_name));
        let connection_string = format!("sqlite://{}?mode=rwc", db_path.to_str().unwrap());
        Database::connect(&connection_string).await
    }

    pub fn get_data_dir(&self) -> &str {
        &self.data_dir
    }
}
