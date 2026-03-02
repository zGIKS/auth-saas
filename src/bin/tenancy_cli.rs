use auth_service::iam::tenancy::interfaces::cli::controllers::tenancy_cli_controller::TenancyCliController;
use sea_orm::Database;
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    let mut args = env::args().skip(1).collect::<Vec<String>>();
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    let command = args.remove(0);
    let database_url =
        env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set in environment")?;
    let db = Database::connect(&database_url).await?;

    let tenancy_controller = TenancyCliController::new(db);

    match command.as_str() {
        "tenant-create" => tenancy_controller.create_tenant(&args).await?,
        "tenant-edit" => tenancy_controller.update_tenant(&args).await?,
        "tenant-rotate-keys" => tenancy_controller.rotate_tenant_keys(&args).await?,
        "tenant-delete" => tenancy_controller.delete_tenant(&args).await?,
        "help" | "--help" | "-h" => print_help(),
        _ => {
            print_help();
            return Err(format!("Unknown command: {command}").into());
        }
    }

    Ok(())
}

fn print_help() {
    println!(
        "tenancy_cli commands:
  tenant-create       --name <name> [--frontend-url <url>] [--google-client-id <id> --google-client-secret <secret>]
  tenant-edit         --tenant-id <uuid> [--name <name>] [--frontend-url <url>] [--google-client-id <id>] [--google-client-secret <secret>]
  tenant-rotate-keys  --tenant-id <uuid>
  tenant-delete       --tenant-id <uuid>
"
    );
}
