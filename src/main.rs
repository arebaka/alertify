use anyhow::{Result, anyhow};
use notify_rust::{Hint, Notification, Timeout, Urgency};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use sysinfo::{DiskExt, System, SystemExt};
use tokio::select;
use tokio::time::{sleep, Duration};
use tokio_stream::StreamExt;
use zbus::{Connection, Proxy, MessageStream};
use zbus::proxy::SignalStream;
use zbus::fdo::{PropertiesProxy, PropertiesChanged};
use zbus::zvariant::{Value, OwnedValue};
use zbus_names::InterfaceName;
use udev::{MonitorBuilder, Event};
use std::os::fd::AsRawFd;
use tokio::io::unix::AsyncFd;

#[derive(Debug, Deserialize, Clone)]
struct Config {
    #[serde(default)]
    battery: Vec<BatteryCase>,
    #[serde(default)]
    ac_online: PowerStatusConfig,
    #[serde(default)]
    ac_offline: PowerStatusConfig,
    #[serde(default)]
    memory: Vec<MemoryCase>,
    #[serde(default)]
    storage: Vec<StorageCase>,
    devices: DevicesConfig,
    network: NetworkConfig,
    bluetooth: BluetoothConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct BatteryCase {
    #[serde(default)]
    level: f64,
    #[serde(default)]
    urgency: String,
    #[serde(default)]
    appname: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    timeout: u32,
    #[serde(default)]
    hints: HashSet<MyHint>,
}

impl Default for BatteryCase {
    fn default() -> Self {
        Self {
            level: 20.0,
            urgency: "critical".to_string(),
            appname: "".to_string(),
            summary: "".to_string(),
            body: "".to_string(),
            icon: "".to_string(),
            timeout: 0,
            hints: vec![].into_iter().collect()
        }
    }
}

impl BatteryCase {
    fn to_message(self) -> Message {
        Message {
            urgency: self.urgency,
            appname: self.appname,
            summary: self.summary,
            body: self.body,
            icon: self.icon,
            timeout: self.timeout,
            hints: self.hints,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
struct PowerStatusConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    urgency: String,
    #[serde(default)]
    appname: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    timeout: u32,
    #[serde(default)]
    hints: HashSet<MyHint>,
}

impl Default for PowerStatusConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            urgency: "critical".to_string(),
            appname: "".to_string(),
            summary: "".to_string(),
            body: "".to_string(),
            icon: "".to_string(),
            timeout: 0,
            hints: vec![].into_iter().collect()
        }
    }
}

impl PowerStatusConfig {
    fn to_message(self) -> Message {
        Message {
            urgency: self.urgency,
            appname: self.appname,
            summary: self.summary,
            body: self.body,
            icon: self.icon,
            timeout: self.timeout,
            hints: self.hints,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
struct MemoryCase {
    #[serde(default)]
    level: f32,
    #[serde(default)]
    urgency: String,
    #[serde(default)]
    appname: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    timeout: u32,
    #[serde(default)]
    hints: HashSet<MyHint>,
}

impl Default for MemoryCase {
    fn default() -> Self {
        Self {
            level: 90.0,
            urgency: "normal".to_string(),
            appname: "".to_string(),
            summary: "".to_string(),
            body: "".to_string(),
            icon: "".to_string(),
            timeout: 0,
            hints: vec![].into_iter().collect()
        }
    }
}

impl MemoryCase {
    fn to_message(self) -> Message {
        Message {
            urgency: self.urgency,
            appname: self.appname,
            summary: self.summary,
            body: self.body,
            icon: self.icon,
            timeout: self.timeout,
            hints: self.hints,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
struct StorageCase {
    #[serde(default)]
    level: f32,
    #[serde(default)]
    urgency: String,
    #[serde(default)]
    appname: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    timeout: u32,
    #[serde(default)]
    hints: HashSet<MyHint>,
}

impl Default for StorageCase {
    fn default() -> Self {
        Self {
            level: 95.0,
            urgency: "normal".to_string(),
            appname: "Storage".to_string(),
            summary: "".to_string(),
            body: "".to_string(),
            icon: "".to_string(),
            timeout: 0,
            hints: vec![].into_iter().collect()
        }
    }
}

impl StorageCase {
    fn to_message(self) -> Message {
        Message {
            urgency: self.urgency,
            appname: self.appname,
            summary: self.summary,
            body: self.body,
            icon: self.icon,
            timeout: self.timeout,
            hints: self.hints,
        }
    }
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

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
struct MyHint(String);

impl From<MyHint> for Hint {
    fn from(hint: MyHint) -> Hint {
        let s = hint.0;
        let parts: Vec<&str> = s.rsplitn(3, ':').collect();

        let (key, value) = match parts.as_slice() {
            // bool:transient:true
            [val, key, "bool"] => (key.to_string(), val.to_string()),
            // int:volume:100
            [val, key, "int"] => (key.to_string(), val.to_string()),
            // double:progress:0.75
            [val, key, "double"] => (key.to_string(), val.to_string()),
            // string:x-dunst-stack-tag:battery.low
            [val, key, "string"] => (key.to_string(), val.to_string()),
            // fallback: no type, just key:value
            [val, key] => (key.to_string(), val.to_string()),
            // just key
            [key] => (key.to_string(), String::new()),
            _ => (s, String::new()),
        };

        Hint::from_key_val(&key, &value).unwrap_or_else(|_| Hint::Custom(key, value))
    }
}

#[derive(Debug, Deserialize, Clone)]
struct Message {
    urgency: String,
    appname: String,
    summary: String,
    body: String,
    icon: String,
    timeout: u32,
    hints: HashSet<MyHint>,
}

impl Message {
    fn notify(self) {
        let urgency = parse_urgency(&self.urgency);
        let mut notification = Notification::new();

        notification
            .summary(&self.summary)
            .body(&self.body)
            .icon(&self.icon)
            .appname(&self.appname)
            .timeout(Timeout::Milliseconds(self.timeout))
            .urgency(urgency);

        for hint in &self.hints {
            notification.hint(hint.clone().into());
        }

        let _ = notification.show();
    }
}

fn parse_urgency(s: &str) -> Urgency {
    match s {
        "low" => Urgency::Low,
        "normal" => Urgency::Normal,
        "critical" => Urgency::Critical,
        _ => Urgency::Normal,
    }
}

async fn monitor_battery(cfg: Vec<BatteryCase>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
    let conn = zbus::Connection::system().await?;
    let properties = PropertiesProxy::builder(&conn)
        .destination("org.freedesktop.UPower")?
        .path("/org/freedesktop/UPower/devices/DisplayDevice")?
        .build()
        .await?;
    let interface = InterfaceName::try_from("org.freedesktop.UPower.Device")?;

    loop {
        let value = properties.get(interface.clone(), "Percentage").await?
            .downcast_ref::<f64>()?
            .to_owned();

        for case in &cfg {
            let should_notify = {
                let key = format!("battery-{}", case.level);
                let mut sent_guard = sent.lock().unwrap();

                if value < case.level {
                    if !sent_guard.contains(&key) {
                        sent_guard.insert(key);
                        true
                    } else {
                        false
                    }
                }
                else {
                    sent_guard.remove(&key);
                    false
                }
            };

            if should_notify {
                let case_clone = case.clone();
                tokio::task::spawn_blocking(move || {
                    case_clone.to_message().notify();
                }).await?;
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }

    Ok(())
}

async fn monitor_memory(cfg: Vec<MemoryCase>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
    loop {
        let mut sys = System::new_all();
        sys.refresh_memory();
        let percent = sys.used_memory() as f32 / sys.total_memory() as f32 * 100.0;

        for case in &cfg {
            let should_notify = {
                let key = format!("memory-{}", case.level);
                let mut sent_guard = sent.lock().unwrap();

                if percent >= case.level {
                    if !sent_guard.contains(&key) {
                        sent_guard.insert(key);
                        true
                    } else {
                        false
                    }
                }
                else {
                    sent_guard.remove(&key);
                    false
                }
            };

            if should_notify {
                let case_clone = case.clone();
                tokio::task::spawn_blocking(move || {
                    case_clone.to_message().notify();
                }).await?;
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}

async fn monitor_storage(cfg: Vec<StorageCase>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
    loop {
        let mut sys = System::new_all();
        sys.refresh_disks_list();

        for disk in sys.disks() {
            let used_percent = 100.0 - (disk.available_space() as f32 / disk.total_space() as f32 * 100.0);
            for case in &cfg {
                let should_notify = {
                    let key = format!("storage-{}-{}", disk.name().to_string_lossy(), case.level);
                    let mut sent_guard = sent.lock().unwrap();

                    if used_percent >= case.level {
                        if !sent_guard.contains(&key) {
                            sent_guard.insert(key);
                            true
                        } else {
                            false
                        }
                    }
                    else {
                        sent_guard.remove(&key);
                        false
                    }
                };

                if should_notify {
                    let case_clone = case.clone();
                    tokio::task::spawn_blocking(move || {
                        case_clone.to_message().notify();
                    }).await?;
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = Path::new("config.toml");
    let config_raw = fs::read_to_string(config_path)?;
    let cfg: Config = toml::from_str(&config_raw)?;

    let sent = Arc::new(Mutex::new(HashSet::new()));
    let mut handles = vec![];

    handles.push(tokio::spawn(monitor_battery(cfg.battery.clone(), sent.clone())));
    handles.push(tokio::spawn(monitor_memory(cfg.memory.clone(), sent.clone())));
    handles.push(tokio::spawn(monitor_storage(cfg.storage.clone(), sent.clone())));
/*
    handles.push(tokio::spawn(monitor_udev(cfg.devices.clone(), sent.clone())));
    handles.push(tokio::spawn(monitor_network(cfg.network.clone(), sent.clone())));
    handles.push(tokio::spawn(monitor_bluetooth(cfg.bluetooth.clone(), sent.clone())));
*/
    for h in handles {
        let _ = h.await;
    }

    Ok(())
}
