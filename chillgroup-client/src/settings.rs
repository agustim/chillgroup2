use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    pub vault: VaultSettings,
    pub notifications: NotificationSettings,
    pub ui: UiSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSettings {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub sound: bool,
    pub mention_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    /// "dark" | "light" | "system"
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: ServerSettings { url: String::new() },
            vault: VaultSettings { path: default_vault_path() },
            notifications: NotificationSettings {
                enabled: true,
                sound: true,
                mention_only: false,
            },
            ui: UiSettings { theme: "dark".to_string() },
        }
    }
}

pub fn config_path() -> PathBuf {
    config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("chillgroup")
        .join("config.toml")
}

pub fn default_vault_path() -> PathBuf {
    config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("chillgroup")
        .join("vault.db")
}

pub fn load() -> Settings {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(settings) = toml::from_str(&content) {
            return settings;
        }
    }
    Settings::default()
}

pub fn save(settings: &Settings) -> anyhow::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(settings)?;
    std::fs::write(&path, content)?;
    Ok(())
}
