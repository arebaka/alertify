use anyhow::{Context, Result};
use serde::Deserialize;
use std::{env, fs, path::{Path, PathBuf}};
use log::info;

use crate::message::Message;

const DEFAULT_CONFIG: &str = include_str!("../config.example.toml");
const CONFIG_FILE_NAME: &str = "config.toml";
const CONFIG_DIR_NAME: &str = "alertify";

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

impl Default for Config {
    fn default() -> Self {
        Self {
            battery: vec![BatteryRule::default()],
            power_supply: vec![PowerStatusRule::default()],
            cpu: vec![CPURule::default()],
            memory: vec![MemoryRule::default()],
            storage: vec![StorageRule::default()],
            device: vec![DeviceRule::default()],
        }
    }
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
    let config_dir = get_config_dir()?;
    Ok(config_dir.join(CONFIG_FILE_NAME))
}

fn get_config_dir() -> Result<PathBuf> {
    let xdg_config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            dirs::home_dir().map(|home| home.join(".config"))
        })
        .context("Could not determine configuration directory")?;

    Ok(xdg_config_home.join(CONFIG_DIR_NAME))
}

fn ensure_config_exists(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    info!("Configuration file not found, creating default at: {}", path.display());

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    fs::write(path, DEFAULT_CONFIG)
        .with_context(|| format!("Failed to write default config to {}", path.display()))?;

    info!("Default configuration created successfully");
    Ok(())
}

pub fn get_config() -> Result<Config> {
    let config_path = get_config_path()
        .context("Failed to determine configuration file")?;

    ensure_config_exists(&config_path)
        .context("Failed to ensure configuration file exists")?;

   let config_content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    let config: Config = toml::from_str(&config_content)
       .with_context(|| format!("Failed to parse configuration file: {}", config_path.display()))?;

    validate_config(&config)?;

    info!("Configuration loaded successfully from: {}", config_path.display());
    Ok(config)
}

fn validate_config(config: &Config) -> Result<()> {
    // Validate battery levels
    for (i, rule) in config.battery.iter().enumerate() {
        if !(0.0..=100.0).contains(&rule.level) {
            return Err(anyhow::anyhow!(
                "Battery rule {}: level must be between 0 and 100, got {}",
                i, rule.level
            ));
        }
    }

    // Validate CPU levels
    for (i, rule) in config.cpu.iter().enumerate() {
        if !(0.0..=100.0).contains(&rule.level) {
            return Err(anyhow::anyhow!(
                "CPU rule {}: level must be between 0 and 100, got {}",
                i, rule.level
            ));
        }
    }

    // Validate memory levels
    for (i, rule) in config.memory.iter().enumerate() {
        if !(0.0..=100.0).contains(&rule.level) {
            return Err(anyhow::anyhow!(
                "Memory rule {}: level must be between 0 and 100, got {}",
                i, rule.level
            ));
        }
    }

    // Validate storage levels
    for (i, rule) in config.storage.iter().enumerate() {
        if !(0.0..=100.0).contains(&rule.level) {
            return Err(anyhow::anyhow!(
                "Storage rule {}: level must be between 0 and 100, got {}",
                i, rule.level
            ));
        }
    }

    Ok(())
}
