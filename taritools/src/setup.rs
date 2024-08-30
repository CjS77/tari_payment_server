use std::path::Path;

use anyhow::Result;
use clap::{Args, Subcommand};
use sqlx::{
    migrate::{MigrateDatabase, Migrator},
    Sqlite,
};
use tari_payment_engine::{
    db_types::{Role, SerializedTariAddress},
    sqlite::db::db_url,
    traits::AuthManagement,
    SqliteDatabase,
};

/// Setup commands work locally to set up Tari Payment Server. These commands assume that `TPG_DATABASE_URL` is set and
/// pointing to the location of the database.
#[derive(Debug, Subcommand)]
pub enum SetupCommand {
    /// Add a new user to the system with a given set of roles
    AddUser(AddUserParams),
    /// Run the database migrations.
    Migrate(MigrateParams),
}

#[derive(Debug, Args)]
pub struct AddUserParams {
    /// The Tari address of the user to add
    #[arg(short, long)]
    pub address: SerializedTariAddress,
    /// The role to assign to the user. This parameter can be specified multiple times to assign multiple roles.
    /// The available roles are: `user`, `read_all`, `write`, `super_admin`.
    #[arg(short, long)]
    pub role: Vec<Role>,
}

#[derive(Debug, Args)]
pub struct MigrateParams {
    /// The path to the migrations directory. The migrations are embedded in the binary by default, and so this
    /// parameter is optional. If provided, the migrations at <path> will be executed instead.
    #[arg(short, long)]
    pub path: Option<String>,
}

pub async fn handle_setup_command(command: SetupCommand) {
    match command {
        SetupCommand::AddUser(params) => add_user(params).await,
        SetupCommand::Migrate(params) => migrate_db(params).await,
    }
}

async fn add_user(params: AddUserParams) {
    async fn add(params: AddUserParams) -> Result<()> {
        let db = SqliteDatabase::new(1).await?;
        db.assign_roles(params.address.as_address(), &params.role).await?;
        Ok(())
    }
    println!("Adding user with address: {} and roles {:?}", params.address, params.role);
    match add(params).await {
        Ok(_) => println!("User added successfully"),
        Err(e) => println!("Error adding user: {e}"),
    }
}

async fn migrate_db(params: MigrateParams) {
    async fn migrate_embedded() -> Result<()> {
        create_database_if_not_exist().await?;
        println!("Running embedded migrations");
        let db = SqliteDatabase::new(1).await?;
        let pool = db.pool();
        sqlx::migrate!("../tari_payment_engine/src/sqlite/migrations").run(pool).await?;
        Ok(())
    }

    async fn migrate_custom(path: &str) -> Result<()> {
        create_database_if_not_exist().await?;
        println!("Running migrations at: {path}");
        let db = SqliteDatabase::new(1).await?;
        let path = Path::new(path);
        let migrator = Migrator::new(path).await?;
        let pool = db.pool();
        migrator.run(pool).await?;
        Ok(())
    }

    let result = match &params.path {
        Some(path) => migrate_custom(path).await,
        None => migrate_embedded().await,
    };

    match result {
        Ok(_) => println!("Migrations complete"),
        Err(e) => println!("Error running migrations: {e}"),
    }
}

async fn create_database_if_not_exist() -> Result<()> {
    let db = db_url();
    if !database_exists(&db).await? {
        println!("Creating new database at: {db}");
        Sqlite::create_database(&db).await?;
    }
    Ok(())
}

async fn database_exists(db: &str) -> Result<bool> {
    let result = Sqlite::database_exists(db).await?;
    Ok(result)
}
