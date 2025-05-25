use serde::Deserialize;

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
