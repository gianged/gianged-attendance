//! Configuration management module.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Configuration load result.
#[derive(Debug)]
pub enum ConfigLoadResult {
    /// Config loaded successfully.
    Loaded(AppConfig),
    /// Config file missing (first run).
    Missing,
    /// Config file exists but invalid.
    Invalid(ConfigError),
}

/// Configuration errors.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    Read(#[from] std::io::Error),

    #[error("Failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("Failed to serialize config: {0}")]
    Serialize(#[from] toml::ser::Error),

    #[error("Validation failed: {0}")]
    Validation(String),
}

/// Main application configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub device: DeviceConfig,
    pub database: DatabaseConfig,
    pub sync: SyncConfig,
    pub ui: UiConfig,
}

/// ZKTeco device connection settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub url: String,
    pub username: String,
    pub password: String,
    /// TCP port for binary protocol (default: 4370).
    #[serde(default = "default_tcp_port")]
    pub tcp_port: u16,
    /// TCP operation timeout in seconds (default: 30).
    #[serde(default = "default_tcp_timeout_secs")]
    pub tcp_timeout_secs: u64,
}

fn default_tcp_port() -> u16 {
    4370
}

fn default_tcp_timeout_secs() -> u64 {
    30
}

/// PostgreSQL database connection settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub username: String,
    pub password: String,
}

/// Sync operation settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub days: i32,
    pub max_user_id: i32,
    pub auto_enabled: bool,
    pub interval_minutes: u32,
}

/// UI preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub start_minimized: bool,
    pub minimize_to_tray: bool,
}

impl AppConfig {
    /// Get config file path (same directory as executable).
    pub fn default_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("config.toml")
    }

    /// Attempt to load config with detailed result.
    pub fn try_load(path: &Path) -> ConfigLoadResult {
        if !path.exists() {
            return ConfigLoadResult::Missing;
        }

        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<AppConfig>(&content) {
                Ok(config) => match config.validate() {
                    Ok(()) => ConfigLoadResult::Loaded(config),
                    Err(e) => ConfigLoadResult::Invalid(e),
                },
                Err(e) => ConfigLoadResult::Invalid(ConfigError::Parse(e)),
            },
            Err(e) => ConfigLoadResult::Invalid(ConfigError::Read(e)),
        }
    }

    /// Validate configuration values.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.database.host.trim().is_empty() {
            return Err(ConfigError::Validation("Database host cannot be empty".to_string()));
        }
        if self.database.port == 0 {
            return Err(ConfigError::Validation(
                "Database port must be greater than 0".to_string(),
            ));
        }
        if self.database.name.trim().is_empty() {
            return Err(ConfigError::Validation("Database name cannot be empty".to_string()));
        }
        if !self.device.url.is_empty() && !self.device.url.starts_with("http") {
            return Err(ConfigError::Validation(
                "Device URL must start with http:// or https://".to_string(),
            ));
        }
        if self.sync.days < 1 {
            return Err(ConfigError::Validation("Sync days must be at least 1".to_string()));
        }
        if self.sync.days > 365 {
            return Err(ConfigError::Validation("Sync days cannot exceed 365".to_string()));
        }
        if self.sync.max_user_id < 1 {
            return Err(ConfigError::Validation("Max user ID must be at least 1".to_string()));
        }
        if self.sync.interval_minutes < 1 {
            return Err(ConfigError::Validation(
                "Sync interval must be at least 1 minute".to_string(),
            ));
        }
        if self.device.tcp_port == 0 {
            return Err(ConfigError::Validation("TCP port must be greater than 0".to_string()));
        }
        if self.device.tcp_timeout_secs < 5 {
            return Err(ConfigError::Validation(
                "TCP timeout must be at least 5 seconds".to_string(),
            ));
        }
        Ok(())
    }

    /// Save configuration to file.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl DatabaseConfig {
    /// Build connection string for SeaORM.
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.name
        )
    }
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            url: "http://192.168.90.11".to_string(),
            username: "administrator".to_string(),
            password: String::new(),
            tcp_port: default_tcp_port(),
            tcp_timeout_secs: default_tcp_timeout_secs(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            name: "gianged_attendance".to_string(),
            username: "postgres".to_string(),
            password: String::new(),
        }
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            days: 30,
            max_user_id: 300,
            auto_enabled: false,
            interval_minutes: 60,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            start_minimized: false,
            minimize_to_tray: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validates() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_connection_string() {
        let db = DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            name: "testdb".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
        };
        assert_eq!(db.connection_string(), "postgres://user:pass@localhost:5432/testdb");
    }

    #[test]
    fn test_validation_empty_host() {
        let mut config = AppConfig::default();
        config.database.host = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_invalid_port() {
        let mut config = AppConfig::default();
        config.database.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_invalid_device_url() {
        let mut config = AppConfig::default();
        config.device.url = "ftp://invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_sync_days_bounds() {
        let mut config = AppConfig::default();

        config.sync.days = 0;
        assert!(config.validate().is_err());

        config.sync.days = 366;
        assert!(config.validate().is_err());

        config.sync.days = 30;
        assert!(config.validate().is_ok());
    }
}
