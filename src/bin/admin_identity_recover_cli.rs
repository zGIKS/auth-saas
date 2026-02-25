use asphanyx::iam::admin_identity::infrastructure::persistence::sqlite::model::{
    ActiveModel, Column, Entity as AdminAccountEntity,
};
use bcrypt::{DEFAULT_COST, hash};
use chrono::Utc;
use dotenvy::dotenv;
use rand::RngCore;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, EntityTrait, PaginatorTrait,
    QueryFilter, Schema, Set, DbErr,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    dotenv().ok();

    if let Err(error) = recover_admin_access().await {
        eprintln!("recover failed: {error}");
        std::process::exit(1);
    }
}

async fn recover_admin_access() -> Result<(), String> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?;

    let database = Database::connect(&database_url)
        .await
        .map_err(|error: DbErr| error.to_string())?;

    let builder = database.get_database_backend();
    let schema = Schema::new(builder);
    let mut create_admin_accounts_table_op = schema.create_table_from_entity(AdminAccountEntity);
    let stmt_admin_accounts = builder.build(create_admin_accounts_table_op.if_not_exists());

    database
        .execute(stmt_admin_accounts)
        .await
        .map_err(|error: DbErr| error.to_string())?;

    let admin_count = AdminAccountEntity::find()
        .count(&database)
        .await
        .map_err(|error: DbErr| error.to_string())?;

    if admin_count > 1 {
        return Err("multiple admin accounts found; manual cleanup required".to_string());
    }

    let username = generate_admin_username();
    let username_hash = hash_admin_username(&username);
    let password = generate_admin_password();
    let password_hash = hash(&password, DEFAULT_COST).map_err(|error| error.to_string())?;
    let now = Utc::now();

    if admin_count == 0 {
        let new_admin = ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            username: Set(username_hash.clone()),
            password_hash: Set(password_hash),
            created_at: Set(now),
            updated_at: Set(now),
        };

        new_admin
            .insert(&database)
            .await
            .map_err(|error: DbErr| error.to_string())?;
    } else {
        let existing_admin = AdminAccountEntity::find()
            .filter(Column::Id.is_not_null())
            .one(&database)
            .await
            .map_err(|error: DbErr| error.to_string())?
            .ok_or("expected admin account but none found")?;

        let updated_admin = ActiveModel {
            id: Set(existing_admin.id),
            username: Set(username_hash),
            password_hash: Set(password_hash),
            created_at: Set(existing_admin.created_at),
            updated_at: Set(now),
        };

        updated_admin
            .update(&database)
            .await
            .map_err(|error: DbErr| error.to_string())?;
    }

    println!("username={username}");
    println!("password={password}");

    Ok(())
}

fn generate_admin_username() -> String {
    let mut bytes = [0u8; 8];
    rand::rng().fill_bytes(&mut bytes);
    format!("admin_{}", hex::encode(bytes))
}

fn generate_admin_password() -> String {
    let mut bytes = [0u8; 24];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn hash_admin_username(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}
