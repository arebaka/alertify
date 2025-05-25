use anyhow::Result;
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}, time::Duration};
use tokio::{task, time::sleep};
use zbus::fdo::PropertiesProxy;
use zbus_names::InterfaceName;

use crate::{config::BatteryCase, utils::maybe_exec};

pub async fn monitor_battery(cfg: Vec<BatteryCase>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
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
                task::spawn_blocking(move || {
                    case_clone.message.notify(&fields);
                })
                .await?;
            }
        }

        sleep(Duration::from_secs(10)).await;
    }
}
