use log::*;
use shopify_payment_gateway::db::Database;
use sqlx::{migrate, migrate::MigrateDatabase, Sqlite};
use std::env;
use std::path::{Path, PathBuf};

pub async fn prepare_test_env() {
    dotenvy::from_filename(".env.test").ok();
    env_logger::init();
    debug!("🚀 Logging initialised");
    let path = db_path();
    create_database(&path).await;
    run_migrations().await;
}

pub fn db_path() -> PathBuf {
    let path = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/test.db".to_string());
    PathBuf::from(path)
}

async fn run_migrations() {
    let pool = Database::new()
        .await
        .expect("Error creating connection to database");
    migrate!("./src/db/sqlite/migrations")
        .run(pool.pool())
        .await
        .expect("Error running DB migrations");
}

async fn create_database<P: AsRef<Path>>(path: P) {
    let p = path.as_ref().as_os_str().to_str().unwrap();
    Sqlite::drop_database(p)
        .await
        .expect("Error dropping database");
    Sqlite::create_database(p)
        .await
        .expect("Error creating database");
    info!("🚀  Created test.db");
}
