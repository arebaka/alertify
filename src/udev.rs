use anyhow::Result;
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};
use tokio::task;
use tokio_stream::StreamExt;
use tokio_udev::{AsyncMonitorSocket, EventType, MonitorBuilder, Device};

use crate::{config::{Config, PowerStatusCase}, utils::maybe_exec};

pub async fn listen_udev(cfg: Config, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
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
            EventType::Change => "change",
            _ => continue,
        };

        if subsystem.clone().unwrap() == *"power_supply" && action == "change" {
            let _ = handle_power_supply_change(event.clone(), cfg.clone().power_supply, sent.clone()).await;
            continue;
        }

        for case in cfg.device.iter()
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
            task::spawn_blocking(move || {
                case_clone.message.notify(
                    &fields
                        .iter()
                        .map(|(&k, v)| (k, v.clone().unwrap_or_default()))
                        .collect(),
                );
            });
        }
    }

    Ok(())
}

async fn handle_power_supply_change(event: Device, cases: Vec<PowerStatusCase>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
    let name = event
        .property_value("POWER_SUPPLY_NAME")
        .and_then(|s| s.to_str())
        .map(str::to_string);
    let supply_type = event
        .property_value("POWER_SUPPLY_TYPE")
        .and_then(|s| s.to_str())
        .map(str::to_string);
    let online = event
        .property_value("POWER_SUPPLY_ONLINE")
        .and_then(|s| s.to_str())
        .map(str::to_string);

    for case in cases.into_iter()
        .filter(|case| match (&case.name, &name) {
            (None, _) | (_, None) => true,
            (Some(expect), Some(actual)) => expect == actual,
        })
        .filter(|case| match (&case.supply_type, &supply_type) {
            (None, _) | (_, None) => true,
            (Some(expect), Some(actual)) => expect == actual,
        })
        .filter(|case| match (&case.online, &online) {
            (None, _) | (_, None) => true,
            (Some(expect), Some(actual)) => expect == actual,
        })
    {
        let mut fields = HashMap::new();
        fields.insert("name", name.clone());
        fields.insert("type", supply_type.clone());
        fields.insert("online", online.clone());

        maybe_exec(case.message.exec.as_ref());
        task::spawn_blocking(move || {
            case.message.notify(
                &fields
                    .iter()
                    .map(|(&k, v)| (k, v.clone().unwrap_or_default()))
                    .collect(),
            );
        })
        .await?;
    }

    Ok(())
}
