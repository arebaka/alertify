use anyhow::Result;
use notify_rust::{Hint, Notification, Timeout, Urgency};
use regex::{Captures, Regex};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use sysinfo::{DiskKind, Disks, System};
use tokio::process::Command;
use tokio_stream::StreamExt;
use tokio_udev::{AsyncMonitorSocket, EventType, MonitorBuilder};
use zbus::fdo::PropertiesProxy;
use zbus_names::InterfaceName;

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
    #[serde(default)]
    device: Vec<DeviceCase>,
    network: NetworkConfig,
    bluetooth: BluetoothConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
struct BatteryCase {
    level: f64,
    #[serde(flatten)]
    message: Message,
}

impl Default for BatteryCase {
    fn default() -> Self {
        Self {
            level: 20.0,
            message: Message {
                urgency: "critical".to_string(),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
struct PowerStatusConfig {
    enabled: bool,
    #[serde(flatten)]
    message: Message,
}

impl Default for PowerStatusConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            message: Message {
                urgency: "critical".to_string(),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
struct MemoryCase {
    level: f32,
    #[serde(flatten)]
    message: Message,
}

impl Default for MemoryCase {
    fn default() -> Self {
        Self {
            level: 90.0,
            message: Message {
                urgency: "normal".to_string(),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
struct StorageCase {
    level: f32,
    #[serde(flatten)]
    message: Message,
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
struct DeviceCase {
    action: String,
    initialized: Option<bool>,
    subsystem: Option<String>,
    sysname: Option<String>,
    sysnum: Option<i32>,
    devtype: Option<String>,
    driver: Option<String>,
    #[serde(flatten)]
    message: Message,
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
                urgency: "normal".to_string(),
                appname: "Device".to_string(),
                ..Default::default()
            },
        }
    }
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
        let s = hint.0.as_str();
        let parts: Vec<&str> = s.rsplitn(3, ':').collect();

        let (key, value) = match *parts.as_slice() {
            // bool:transient:true
            [val, key, "bool"] => (key, val),
            // int:volume:100
            [val, key, "int"] => (key, val),
            // double:progress:0.75
            [val, key, "double"] => (key, val),
            // string:x-dunst-stack-tag:battery.low
            [val, key, "string"] => (key, val),
            // fallback: no type, just key:value
            [val, key] => (key, val),
            // just key
            [key] => (key, ""),
            _ => (s, ""),
        };

        Hint::from_key_val(key, value).unwrap_or(Hint::Custom(key.to_string(), value.to_string()))
    }
}

#[derive(Default, Debug, Deserialize, Clone)]
struct Message {
    urgency: String,
    appname: String,
    summary: String,
    body: String,
    icon: String,
    timeout: u32,
    hints: HashSet<MyHint>,
    #[serde(default)]
    exec: Option<String>,
}

impl Message {
    fn render(template: &str, fields: &HashMap<&str, String>) -> String {
        let re = Regex::new(r"\{([a-zA-Z0-9_]+)\}").unwrap();
        re.replace_all(template, |caps: &Captures| {
            let key = &caps[1];
            fields
                .get(key)
                .cloned()
                .unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned()
    }

    fn notify(&self, fields: &HashMap<&str, String>) {
        let urgency = parse_urgency(&self.urgency);
        let mut notification = Notification::new();

        notification
            .urgency(urgency)
            .appname(&Self::render(&self.appname, fields))
            .summary(&Self::render(&self.summary, fields))
            .body(&Self::render(&self.body, fields))
            .icon(&self.icon)
            .timeout(Timeout::Milliseconds(self.timeout));

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

fn maybe_exec(exec: Option<&String>) {
    if let Some(cmdline) = exec {
        let status = Command::new("sh").arg("-c").arg(cmdline).spawn();
    }
}
/*
async fn listen_property_changes(cfg: &Config) -> Result<()> {
    let conn = Connection::system().await?;
    let mut stream = MessageStream::from(&conn);

    while let Some(msg) = stream.next().await {
        let msg = msg?;
        let (interface, changed, _): (
            InterfaceName<'_>,
            HashMap<String, OwnedValue>,
            Vec<String>,
        ) = msg.body().try_into()?;

        match interface.as_str() {
            "org.freedesktop.UPower.Device" => {
                handle_power_connection_change(cfg, changed).await?
            }
            _ => {}
        }
    }

    Ok(())
}

async fn handle_power_connection_change(cfg: &Config, changed: HashMap<String, OwnedValue>) -> Result<()> {
    if let Some(value) = changed.get("Online") {
        let mut case: PowerStatusConfig;
        if let Some(is_online) = value.downcast_ref::<bool>() {
            let case = if *is_online {
                cfg.ac_offline.clone()
            } else {
                cfg.ac_offline.clone()
            };

            let case_clone = case.clone();
            tokio::task::spawn_blocking(move || {
                case_clone.to_message().notify();
            }).await?;
        }
        else {
            return Err(anyhow!("Failed to downcast Online to bool"));
        }
    }

    Ok(())
}
*/
async fn monitor_battery(cfg: Vec<BatteryCase>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
    let conn = zbus::Connection::system().await?;
    let properties = PropertiesProxy::builder(&conn)
        .destination("org.freedesktop.UPower")?
        .path("/org/freedesktop/UPower/devices/DisplayDevice")?
        .build()
        .await?;
    let interface = InterfaceName::try_from("org.freedesktop.UPower.Device")?;

    loop {
        let value = properties
            .get(interface.clone(), "Percentage")
            .await?
            .downcast_ref::<f64>()?
            .to_owned();

        for case in &cfg {
            let should_notify = {
                let key = format!("battery-{}", case.level);
                let mut sent_guard = sent.lock().unwrap();

                if value < case.level {
                    if sent_guard.contains(&key) {
                        false
                    } else {
                        sent_guard.insert(key);
                        true
                    }
                } else {
                    sent_guard.remove(&key);
                    false
                }
            };

            if should_notify {
                let mut fields = HashMap::new();
                fields.insert("level", case.level.to_string());
                fields.insert("left_percent", (value as u32).to_string());
                fields.insert("used_percent", (100 - value as u32).to_string());

                let case_clone = case.clone();
                maybe_exec(case_clone.message.exec.as_ref());
                tokio::task::spawn_blocking(move || {
                    case_clone.message.notify(&fields);
                })
                .await?;
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}

async fn monitor_memory(cfg: Vec<MemoryCase>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
    loop {
        let mut sys = System::new_all();
        sys.refresh_memory();

        let total = sys.total_memory();
        let used = sys.used_memory();
        let left = total - used;
        let used_percent = used as f32 / total as f32 * 100.0;
        let left_percent = 100.0 - used_percent;

        for case in &cfg {
            let should_notify = {
                let key = format!("memory-{}", case.level);
                let mut sent_guard = sent.lock().unwrap();

                if used_percent >= case.level {
                    if sent_guard.contains(&key) {
                        false
                    } else {
                        sent_guard.insert(key);
                        true
                    }
                } else {
                    sent_guard.remove(&key);
                    false
                }
            };

            if should_notify {
                let mut fields = HashMap::new();
                fields.insert("level", case.level.to_string());
                fields.insert("total_bytes", total.to_string());
                fields.insert("total", humansize::format_size(total, humansize::DECIMAL));
                fields.insert("used_bytes", used.to_string());
                fields.insert("used", humansize::format_size(used, humansize::DECIMAL));
                fields.insert("used_percent_full", used_percent.to_string());
                fields.insert("used_percent", (used_percent as u32).to_string());
                fields.insert("left_bytes", left.to_string());
                fields.insert("left", humansize::format_size(left, humansize::DECIMAL));
                fields.insert("left_percent_full", left_percent.to_string());
                fields.insert("left_percent", (left_percent as u32).to_string());

                let case_clone = case.clone();
                maybe_exec(case_clone.message.exec.as_ref());
                tokio::task::spawn_blocking(move || {
                    case_clone.message.notify(&fields);
                })
                .await?;
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}

async fn monitor_storage(cfg: Vec<StorageCase>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
    loop {
        let disks = Disks::new_with_refreshed_list();

        for disk in disks.list() {
            for case in &cfg {
                let kind = match disk.kind() {
                    DiskKind::HDD => "HDD",
                    DiskKind::SSD => "SSD",
                    _ => "unknown",
                }
                .to_string();

                let name = disk.name().to_string_lossy().into_owned();
                let fs = disk.file_system().to_string_lossy().into_owned();
                let mount = disk.mount_point().to_string_lossy().into_owned();
                let total = disk.total_space();
                let left = disk.available_space();
                let used = total - left;
                let left_percent = left as f32 / total as f32 * 100.0;
                let used_percent = 100.0 - left_percent;

                let should_notify = {
                    let key = format!("storage-{}-{}", mount, case.level);
                    let mut sent_guard = sent.lock().unwrap();

                    if used_percent >= case.level {
                        if sent_guard.contains(&key) {
                            false
                        } else {
                            sent_guard.insert(key);
                            true
                        }
                    } else {
                        sent_guard.remove(&key);
                        false
                    }
                };

                if should_notify {
                    let mut fields = HashMap::new();
                    fields.insert("level", case.level.to_string());
                    fields.insert("kind", kind);
                    fields.insert("name", name);
                    fields.insert("fs", fs);
                    fields.insert("mount", mount);
                    fields.insert("total_bytes", total.to_string());
                    fields.insert("total", humansize::format_size(total, humansize::DECIMAL));
                    fields.insert("used_bytes", used.to_string());
                    fields.insert("used", humansize::format_size(used, humansize::DECIMAL));
                    fields.insert("used_percent_full", used_percent.to_string());
                    fields.insert("value_percent", (used_percent as u32).to_string());
                    fields.insert("left_bytes", left.to_string());
                    fields.insert("left", humansize::format_size(left, humansize::DECIMAL));
                    fields.insert("left_percent_full", left_percent.to_string());
                    fields.insert("left_percent", (left_percent as u32).to_string());

                    let case_clone = case.clone();
                    maybe_exec(case_clone.message.exec.as_ref());
                    tokio::task::spawn_blocking(move || {
                        case_clone.message.notify(&fields);
                    })
                    .await?;
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

async fn listen_udev(cases: Vec<DeviceCase>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
    const ALLOW_SUBSYSTEMS: &[&str] = &[
        "usb",
        "block",
        "net",
        "input",
        "sound",
        "drm",
        "tty",
        "power_supply",
        "video4linux",
    ];

    let mut monitor = MonitorBuilder::new()?;
    for subsys in ALLOW_SUBSYSTEMS {
        monitor = monitor.match_subsystem(subsys)?;
    }
    let mut socket = AsyncMonitorSocket::new(monitor.listen()?)?;

    while let Some(Ok(event)) = socket.next().await {
        let initialized = event.is_initialized();
        let subsystem = event
            .subsystem()
            .and_then(|s| s.to_str().map(str::to_string));
        let sysname = event.sysname().to_str().map(str::to_string);
        let sysnum = event.sysnum().map(|n| n as i32);
        let devtype = event.devtype().and_then(|s| s.to_str().map(str::to_string));
        let driver = event.driver().and_then(|s| s.to_str().map(str::to_string));

        let action = match event.event_type() {
            EventType::Add => "add",
            EventType::Remove => "remove",
            EventType::Bind => "bind",
            EventType::Unbind => "unbind",
            _ => continue,
        };

        for case in cases
            .iter()
            .filter(|case| case.action == action)
            .filter(|case| case.initialized.is_none_or(|v| v == initialized))
            .filter(|case| match (&case.subsystem, &subsystem) {
                (None, _) | (_, None) => true,
                (Some(expect), Some(actual)) => expect == actual,
            })
            .filter(|case| match (&case.sysname, &sysname) {
                (None, _) | (_, None) => true,
                (Some(expect), Some(actual)) => expect == actual,
            })
            .filter(|case| case.sysnum.is_none_or(|v| sysnum == Some(v)))
            .filter(|case| match (&case.devtype, &devtype) {
                (None, _) | (_, None) => true,
                (Some(expect), Some(actual)) => expect == actual,
            })
            .filter(|case| match (&case.driver, &driver) {
                (None, _) | (_, None) => true,
                (Some(expect), Some(actual)) => expect == actual,
            })
        {
            let mut fields = HashMap::new();
            let subsystem = event
                .subsystem()
                .and_then(|s| s.to_str().map(str::to_string));
            let sysname = event.sysname().to_str().map(str::to_string);
            let sysnum = event.sysnum().map(|n| n as i32);
            let devtype = event.devtype().and_then(|s| s.to_str().map(str::to_string));
            let driver = event.driver().and_then(|s| s.to_str().map(str::to_string));
            let seq_num = Some(event.sequence_number().to_string());
            let syspath = event.syspath().to_str().map(str::to_string);
            let devpath = event.devpath().to_str().map(str::to_string);
            let devnode = event.devnode().and_then(|s| s.to_str().map(str::to_string));

            fields.insert("subsystem", subsystem);
            fields.insert("sysname", sysname);
            fields.insert("sysnum", sysnum.map(|n| n.to_string()));
            fields.insert("devtype", devtype);
            fields.insert("driver", driver);
            fields.insert("seq_num", seq_num);
            fields.insert("syspath", syspath);
            fields.insert("devpath", devpath);
            fields.insert("devnode", devnode);

            let case_clone = case.clone();
            maybe_exec(case_clone.message.exec.as_ref());
            tokio::task::spawn_blocking(move || {
                case_clone.message.notify(
                    &fields
                        .iter()
                        .map(|(&k, v)| (k, v.clone().unwrap_or_default()))
                        .collect(),
                );
            })
            .await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = Path::new("config.toml");
    let config_raw = fs::read_to_string(config_path)?;
    let cfg: Config = toml::from_str(&config_raw)?;

    let sent = Arc::new(Mutex::new(HashSet::new()));
    let handles = vec![
        tokio::spawn(monitor_battery(cfg.battery.clone(), sent.clone())),
        tokio::spawn(monitor_memory(cfg.memory.clone(), sent.clone())),
        tokio::spawn(monitor_storage(cfg.storage.clone(), sent.clone())),
        // tokio::spawn(monitor_network(cfg.network.clone(), sent.clone())),
        // tokio::spawn(monitor_bluetooth(cfg.bluetooth.clone(), sent.clone())),
    ];
    listen_udev(cfg.device.clone(), sent.clone()).await.unwrap();
    for h in handles {
        let _ = h.await;
    }

    Ok(())
}
