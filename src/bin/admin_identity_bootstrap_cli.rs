use auth_service::iam::{
    admin_identity::{
        application::{
            command_services::admin_identity_command_service_impl::AdminIdentityCommandServiceImpl,
            query_services::admin_identity_query_service_impl::AdminIdentityQueryServiceImpl,
        },
        domain::{
            model::commands::create_initial_admin_command::CreateInitialAdminCommand,
            services::admin_identity_command_service::AdminIdentityCommandService,
        },
        infrastructure::persistence::sqlite::{
            model::Entity as AdminAccountEntity,
            repositories::admin_account_repository_impl::AdminAccountRepositoryImpl,
        },
    },
    authentication::infrastructure::services::jwt_token_service::JwtTokenService,
};
use dotenvy::dotenv;
use rand::RngCore;
use sea_orm::{ConnectionTrait, Database, Schema};

#[tokio::main]
async fn main() {
    dotenv().ok();

    if let Err(error) = bootstrap_initial_admin().await {
        eprintln!("bootstrap failed: {error}");
        std::process::exit(1);
    }
}

async fn bootstrap_initial_admin() -> Result<(), String> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?;
    let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| "JWT_SECRET must be set")?;

    let database = Database::connect(&database_url)
        .await
        .map_err(|error| error.to_string())?;

    let builder = database.get_database_backend();
    let schema = Schema::new(builder);
    let mut create_admin_accounts_table_op = schema.create_table_from_entity(AdminAccountEntity);
    let stmt_admin_accounts = builder.build(create_admin_accounts_table_op.if_not_exists());

    database
        .execute(stmt_admin_accounts)
        .await
        .map_err(|error| error.to_string())?;

    let username = generate_admin_username();
    let password = generate_admin_password();

    let command = CreateInitialAdminCommand::new(username.clone(), password.clone())
        .map_err(|error| error.to_string())?;

    let query_repository = AdminAccountRepositoryImpl::new(database.clone());
    let query_service = AdminIdentityQueryServiceImpl::new(query_repository);

    let command_repository = AdminAccountRepositoryImpl::new(database);
    let token_service = JwtTokenService::new(jwt_secret, 3600);
    let command_service =
        AdminIdentityCommandServiceImpl::new(command_repository, query_service, token_service);

    command_service
        .handle_create_initial_admin(command)
        .await
        .map_err(|error| error.to_string())?;

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
