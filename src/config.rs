use anyhow::{Context, Result};
use serde::Deserialize;
use std::{env, fs, path::{Path, PathBuf}};

use crate::message::Message;

const DEFAULT_CONFIG: &str = include_str!("../config.example.toml");

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub battery: Vec<BatteryRule>,
    #[serde(default)]
    pub power_supply: Vec<PowerStatusRule>,
    #[serde(default)]
    pub cpu: Vec<CPURule>,
    #[serde(default)]
    pub memory: Vec<MemoryRule>,
    #[serde(default)]
    pub storage: Vec<StorageRule>,
    #[serde(default)]
    pub device: Vec<DeviceRule>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct BatteryRule {
    pub level: f64,
    #[serde(flatten)]
    pub message: Message,
}

impl Default for BatteryRule {
    fn default() -> Self {
        Self {
            level: 20.0,
            message: Message {
                urgency: "critical".to_string(),
                appname: "Battery".to_string(),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct CPURule {
    pub level: f32,
    #[serde(flatten)]
    pub message: Message,
}

impl Default for CPURule {
    fn default() -> Self {
        Self {
            level: 90.0,
            message: Message {
                urgency: "normal".to_string(),
                appname: "CPU".to_string(),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct MemoryRule {
    pub level: f32,
    #[serde(flatten)]
    pub message: Message,
}

impl Default for MemoryRule {
    fn default() -> Self {
        Self {
            level: 90.0,
            message: Message {
                urgency: "normal".to_string(),
                appname: "Memory".to_string(),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct StorageRule {
    pub level: f32,
    #[serde(flatten)]
    pub message: Message,
}

impl Default for StorageRule {
    fn default() -> Self {
        Self {
            level: 95.0,
            message: Message {
                urgency: "normal".to_string(),
                appname: "Storage".to_string(),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DeviceRule {
    pub action: String,
    pub initialized: Option<bool>,
    pub subsystem: Option<String>,
    pub sysname: Option<String>,
    pub sysnum: Option<i32>,
    pub devtype: Option<String>,
    pub driver: Option<String>,
    #[serde(flatten)]
    pub message: Message,
}

impl Default for DeviceRule {
    fn default() -> Self {
        Self {
            action: "add".to_string(),
            initialized: None,
            subsystem: None,
            sysname: None,
            sysnum: None,
            devtype: None,
            driver: None,
            message: Message {
                urgency: "low".to_string(),
                appname: "Device".to_string(),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct PowerStatusRule {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub supply_type: Option<String>,
    pub online: Option<String>,
    #[serde(flatten)]
    pub message: Message,
}

impl Default for PowerStatusRule {
    fn default() -> Self {
        Self {
            name: None,
            supply_type: None,
            online: None,
            message: Message {
                urgency: "low".to_string(),
                appname: "Power supply".to_string(),
                ..Default::default()
            },
        }
    }
}

fn get_config_path() -> Result<PathBuf> {
    let xdg_config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|home| home.join(".config"))
                .expect("Could not determine home directory")
        });

    let path = xdg_config_home.join("alertify").join("config.toml");
    Ok(path)
}

fn ensure_config_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        fs::write(path, DEFAULT_CONFIG)
            .with_context(|| format!("Failed to write default config to {}", path.display()))?;
    }

    Ok(())
}

pub fn get_config() -> Result<Config> {
    let config_path = get_config_path()
        .context("Failed to locate configuration file")?;

    ensure_config_exists(&config_path)?;

    let config_raw = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    let config: Config = toml::from_str(&config_raw)
        .context("Failed to parse configuration file")?;

    Ok(config)
}
