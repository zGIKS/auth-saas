use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::Path;

const MIGRATIONS_DIR: &str = "migrations";

pub async fn run_migrations(db: &DatabaseConnection) -> Result<(), Box<dyn Error>> {
    ensure_schema_migrations_table(db).await?;

    let applied_versions = fetch_applied_versions(db).await?;
    let pending = pending_migrations(&applied_versions)?;

    for (version, sql) in pending {
        db.execute_unprepared(&sql).await?;
        record_migration(db, &version).await?;
        println!("Applied migration: {version}");
    }

    Ok(())
}

async fn ensure_schema_migrations_table(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version varchar PRIMARY KEY,
            applied_at timestamptz NOT NULL DEFAULT NOW()
        )",
    )
    .await?;

    Ok(())
}

async fn fetch_applied_versions(
    db: &DatabaseConnection,
) -> Result<HashSet<String>, sea_orm::DbErr> {
    let query = Statement::from_string(
        DbBackend::Postgres,
        "SELECT version FROM schema_migrations".to_string(),
    );

    let rows = db.query_all(query).await?;
    let mut versions = HashSet::with_capacity(rows.len());

    for row in rows {
        let version: String = row.try_get("", "version")?;
        versions.insert(version);
    }

    Ok(versions)
}

fn pending_migrations(
    applied_versions: &HashSet<String>,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let dir = Path::new(MIGRATIONS_DIR);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "sql"))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();

    entries.sort();

    let mut pending = Vec::new();
    for path in entries {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        let version = file_name.to_string();
        if applied_versions.contains(&version) {
            continue;
        }

        let sql = fs::read_to_string(&path)?;
        pending.push((version, sql));
    }

    Ok(pending)
}

async fn record_migration(db: &DatabaseConnection, version: &str) -> Result<(), sea_orm::DbErr> {
    let escaped_version = version.replace('\'', "''");
    let sql = format!("INSERT INTO schema_migrations (version) VALUES ('{escaped_version}')");

    db.execute_unprepared(&sql).await?;
    Ok(())
}
