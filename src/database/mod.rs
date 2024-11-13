// Copyright 2023 Remi Bernotavicius

use diesel::prelude::Connection as _;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::error::Error;
use std::path::Path;

pub mod models;
pub mod schema;

pub type Connection = diesel::sqlite::SqliteConnection;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn establish_connection(
    path: impl AsRef<Path>,
) -> Result<Connection, Box<dyn Error + Send + Sync + 'static>> {
    let mut connection = Connection::establish(path.as_ref().to_str().unwrap())?;
    connection.run_pending_migrations(MIGRATIONS)?;
    Ok(connection)
}

#[test]
fn migrations() {
    use std::process::Command;
    use std::{env, fs};

    let out_dir = env::temp_dir();
    let database_path = out_dir.join("database.sqlite");
    if database_path.exists() {
        fs::remove_file(&database_path).unwrap();
    }

    for cmd in ["run", "redo"] {
        let status = Command::new("diesel")
            .args([
                "migration",
                cmd,
                "--database-url",
                database_path.to_str().unwrap(),
            ])
            .status()
            .unwrap();
        assert!(status.success());
    }

    fs::remove_file(&database_path).unwrap();
}
