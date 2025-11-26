# Phase 05: Configuration

## Objective

Implement configuration loading and saving with TOML format.

---

## Tasks

### 5.1 Create Config Module

**`src/config.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub device: DeviceConfig,
    pub database: DatabaseConfig,
    pub sync: SyncConfig,
    pub ui: UiConfig,
}

/// ZKTeco device connection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub url: String,
    pub username: String,
    pub password: String,
}

/// PostgreSQL database connection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub username: String,
    pub password: String,
}

impl DatabaseConfig {
    /// Build connection string for sqlx
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.name
        )
    }
}

/// Sync operation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub days: i32,
    pub max_user_id: i32,
    pub auto_enabled: bool,
    pub interval_minutes: u32,
}

/// UI preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub start_minimized: bool,
    pub minimize_to_tray: bool,
}

impl AppConfig {
    /// Load configuration from TOML file
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to TOML file
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get default config file path
    pub fn default_path() -> std::path::PathBuf {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        exe_dir.join("config.toml")
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            device: DeviceConfig {
                url: "http://192.168.90.11".to_string(),
                username: "administrator".to_string(),
                password: "123456".to_string(),
            },
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 5432,
                name: "gianged_attendance".to_string(),
                username: "postgres".to_string(),
                password: "password".to_string(),
            },
            sync: SyncConfig {
                days: 30,
                max_user_id: 300,
                auto_enabled: false,
                interval_minutes: 60,
            },
            ui: UiConfig {
                start_minimized: false,
                minimize_to_tray: true,
            },
        }
    }
}
```

### 5.2 Create config.example.toml

```toml
[device]
url = "http://192.168.90.11"
username = "administrator"
password = "123456"

[database]
host = "localhost"
port = 5432
name = "gianged_attendance"
username = "postgres"
password = "password"

[sync]
days = 30
max_user_id = 300
auto_enabled = false
interval_minutes = 60

[ui]
start_minimized = false
minimize_to_tray = true
```

---

## Deliverables

- [x] Config structs with serde
- [x] Load from TOML file
- [x] Save to TOML file
- [x] Default values
- [x] config.example.toml template
