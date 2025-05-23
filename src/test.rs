
use std::collections::HashSet;
use std::ffi::OsStr;
use std::time::Duration;

use notify_rust::{Hint, Notification, Timeout, Urgency};
use serde::Deserialize;
use tokio::task;
use zbus::Connection;

#[derive(Debug, Deserialize, Clone)]
struct Config {
    battery: Vec<BatteryCase>,
    memory: Vec<MemoryCase>,
    storage: Vec<StorageCase>,
    devices: DevicesConfig,
    network: NetworkConfig,
    bluetooth: BluetoothConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct BatteryCase {
    level: u32,
    urgency: String,
    appname: String,
    summary: String,
    body: String,
    icon: String,
    timeout: Timeout,
    hints: HashSet<Hint>,
}

#[derive(Debug, Deserialize, Clone)]
struct MemoryCase {
    percent: f32,
    urgency: String,
    appname: String,
    summary: String,
    body: String,
    icon: String,
    timeout: Timeout,
    hints: HashSet<Hint>,
}

#[derive(Debug, Deserialize, Clone)]
struct StorageCase {
    percent: f32,
    urgency: String,
    appname: String,
    summary: String,
    body: String,
    icon: String,
    timeout: Timeout,
    hints: HashSet<Hint>,
}

#[derive(Debug, Deserialize, Clone)]
struct DevicesConfig {
    usb: DeviceConfig,
    hdmi: DeviceConfig,
    jack: DeviceConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct DeviceConfig {
    connect: Message,
    disconnect: Message,
}

#[derive(Debug, Deserialize, Clone)]
struct NetworkConfig {
    disconnect: Message,
    reconnect: Message,
}

#[derive(Debug, Deserialize, Clone)]
struct BluetoothConfig {
    disconnect: Message,
    reconnect: Message,
}

#[derive(Debug, Deserialize, Clone)]
struct Message {
    urgency: String,
    appname: String,
    summary: String,
    body: String,
    icon: String,
    timeout: Timeout,
    hints: HashSet<Hint>,
}

impl Message {
    fn to_notification(&self) -> Notification {
        let mut notification = Notification::new()
            .appname(&self.appname)
            .summary(&self.summary)
            .body(&self.body)
            .icon(&self.icon)
            .timeout(self.timeout.clone());

        for hint in &self.hints {
            notification = notification.hint(hint.clone());
        }

        if let Ok(urgency) = self.urgency.parse::<Urgency>() {
            notification = notification.urgency(urgency);
        }

        notification
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_data = std::fs::read_to_string("config.toml")?;
    let cfg: Config = toml::from_str(&config_data)?;

    for case in cfg.battery.iter() {
        case.to_notification().show()?;
    }

    Ok(())
}
