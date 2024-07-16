use std::{
    fs,
    io,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use dirs::home_dir;
use log::info;
use serde::{Deserialize, Serialize};
use tari_crypto::ristretto::RistrettoSecretKey;
use tari_payment_engine::db_types::{Role, SerializedTariAddress};

#[derive(Serialize, Deserialize, Default)]
pub struct UserData {
    pub profiles: Vec<Profile>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Profile {
    pub name: String,
    pub address: SerializedTariAddress,
    pub secret_key: RistrettoSecretKey,
    pub roles: Vec<Role>,
    pub server: String,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            name: "default".to_string(),
            address: SerializedTariAddress::default(),
            secret_key: RistrettoSecretKey::default(),
            roles: vec![Role::User],
            server: "http://localhost:4444".to_string(),
        }
    }
}

pub fn get_config_path() -> io::Result<PathBuf> {
    let home = home_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found"))?;
    let config_dir = home.join(".taritools");
    if !config_dir.exists() {
        fs::create_dir(&config_dir)?;
        set_permissions(&config_dir, 0o700)?;
    }
    let config_file = config_dir.join("config.toml");
    if !config_file.exists() {
        info!("Creating default config file");
        let default_config = UserData::default();
        let config_str =
            toml::to_string(&default_config).map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
        fs::write(&config_file, config_str)?;
        set_permissions(&config_file, 0o600)?;
    }
    Ok(config_dir.join("config.toml"))
}

fn set_permissions(config_dir: &PathBuf, perms: u32) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&config_dir)?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(perms); // Sets directory to only be accessible by the owner
        fs::set_permissions(&config_dir, permissions)?;
    }
    Ok(())
}

pub fn read_config() -> io::Result<UserData> {
    let config_path = get_config_path()?;
    let config_str = fs::read_to_string(config_path)?;
    let config: UserData =
        toml::from_str(&config_str).map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
    Ok(config)
}

pub fn write_config(config: &Profile) -> io::Result<()> {
    let config_path = get_config_path()?;
    let config_str = toml::to_string(config).map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
    fs::write(&config_path, config_str)?;
    set_permissions(&config_path, 0o600)?;
    Ok(())
}
