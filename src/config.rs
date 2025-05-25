use serde::Deserialize;
use std::{env, fs, path::PathBuf};
use anyhow::{Context, Result};

use crate::message::Message;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub battery: Vec<BatteryCase>,
    #[serde(default)]
    pub power_supply: Vec<PowerStatusCase>,
    #[serde(default)]
    pub memory: Vec<MemoryCase>,
    #[serde(default)]
    pub storage: Vec<StorageCase>,
    #[serde(default)]
    pub device: Vec<DeviceCase>,
    pub network: NetworkConfig,
    pub bluetooth: BluetoothConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct BatteryCase {
    pub level: f64,
    #[serde(flatten)]
    pub message: Message,
}

impl Default for BatteryCase {
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
pub struct MemoryCase {
    pub level: f32,
    #[serde(flatten)]
    pub message: Message,
}

impl Default for MemoryCase {
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
pub struct StorageCase {
    pub level: f32,
    #[serde(flatten)]
    pub message: Message,
}

impl Default for StorageCase {
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
pub struct DeviceCase {
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

impl Default for DeviceCase {
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
pub struct PowerStatusCase {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub supply_type: Option<String>,
    pub online: Option<String>,
    #[serde(flatten)]
    pub message: Message,
}

impl Default for PowerStatusCase {
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

#[derive(Debug, Deserialize, Clone)]
pub struct NetworkConfig {
    pub disconnect: Message,
    pub reconnect: Message,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BluetoothConfig {
    pub disconnect: Message,
    pub reconnect: Message,
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

    if path.exists() {
        Ok(path)
    } else {
        Err(anyhow::anyhow!("Configuration file not found: {:?}", path))
    }
}

pub fn get_config() -> Result<Config> {
    let config_path = get_config_path()
        .context("Failed to locate configuration file")?;

    let config_raw = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    let config: Config = toml::from_str(&config_raw)
        .context("Failed to parse configuration file")?;

    Ok(config)
}
