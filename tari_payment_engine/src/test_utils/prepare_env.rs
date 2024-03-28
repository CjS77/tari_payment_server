use std::path::Path;

use log::*;
use sqlx::{migrate, migrate::MigrateDatabase, Sqlite};

use crate::SqliteDatabase;

pub async fn prepare_test_env(url: &str) {
    dotenvy::from_filename(".env.test").ok();
    let _ = env_logger::try_init();
    debug!("🚀️ Logging initialised");
    create_database(url).await;
    run_migrations(url).await;
}

pub fn random_db_path() -> String {
    format!("sqlite://../data/test_store_{}", rand::random::<u64>())
}

pub async fn run_migrations(url: &str) {
    let db = SqliteDatabase::new_with_url(url, 5).await.expect("Error creating connection to database");
    migrate!("./src/db/sqlite/migrations").run(db.pool()).await.expect("Error running DB migrations");
    info!("🚀️ Migrations complete");
}

pub async fn create_database<P: AsRef<Path>>(path: P) {
    let p = path.as_ref().as_os_str().to_str().unwrap();
    if let Err(e) = Sqlite::drop_database(p).await {
        warn!("Error dropping database {p}: {e:?}");
    }
    Sqlite::create_database(p).await.expect("Error creating database");
    info!("Created Sqlite database {p}");
}
