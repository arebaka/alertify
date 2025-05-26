use anyhow::Result;
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};
use tokio::task;
use tokio_stream::StreamExt;
use tokio_udev::{AsyncMonitorSocket, EventType, MonitorBuilder, Device};

use crate::{config::{Config, PowerStatusRule}, utils::maybe_exec};

pub async fn listen_udev(rules: Config, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
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
            EventType::Add    => "add",
            EventType::Remove => "remove",
            EventType::Bind   => "bind",
            EventType::Unbind => "unbind",
            EventType::Change => "change",
            _ => continue,
        };

        if subsystem.clone().unwrap() == *"power_supply" && action == "change" {
            let _ = handle_power_supply_change(event.clone(), rules.clone().power_supply, sent.clone()).await;
        }

        for rule in rules.device.iter()
            .filter(|rule| rule.action == action)
            .filter(|rule| rule.initialized.is_none_or(|v| v == initialized))
            .filter(|rule| match (&rule.subsystem, &subsystem) {
                (None, _) | (_, None) => true,
                (Some(expect), Some(actual)) => expect == actual,
            })
            .filter(|rule| match (&rule.sysname, &sysname) {
                (None, _) | (_, None) => true,
                (Some(expect), Some(actual)) => expect == actual,
            })
            .filter(|rule| rule.sysnum.is_none_or(|v| sysnum == Some(v)))
            .filter(|rule| match (&rule.devtype, &devtype) {
                (None, _) | (_, None) => true,
                (Some(expect), Some(actual)) => expect == actual,
            })
            .filter(|rule| match (&rule.driver, &driver) {
                (None, _) | (_, None) => true,
                (Some(expect), Some(actual)) => expect == actual,
            })
        {
            let mut fields = HashMap::new();
            let subsystem = event.subsystem().and_then(|s| s.to_str().map(str::to_string));
            let sysname   = event.sysname().to_str().map(str::to_string);
            let sysnum    = event.sysnum().map(|n| n as i32);
            let devtype   = event.devtype().and_then(|s| s.to_str().map(str::to_string));
            let driver    = event.driver().and_then(|s| s.to_str().map(str::to_string));
            let seq_num   = Some(event.sequence_number().to_string());
            let syspath   = event.syspath().to_str().map(str::to_string);
            let devpath   = event.devpath().to_str().map(str::to_string);
            let devnode   = event.devnode().and_then(|s| s.to_str().map(str::to_string));

            fields.insert("subsystem", subsystem);
            fields.insert("sysname",   sysname);
            fields.insert("sysnum",    sysnum.map(|n| n.to_string()));
            fields.insert("devtype",   devtype);
            fields.insert("driver",    driver);
            fields.insert("seq_num",   seq_num);
            fields.insert("syspath",   syspath);
            fields.insert("devpath",   devpath);
            fields.insert("devnode",   devnode);

            let rule_clone = rule.clone();
            maybe_exec(rule_clone.message.exec.as_ref());
            task::spawn_blocking(move || {
                rule_clone.message.notify(
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

async fn handle_power_supply_change(event: Device, rules: Vec<PowerStatusRule>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
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

    for rule in rules.into_iter()
        .filter(|rule| match (&rule.name, &name) {
            (None, _) | (_, None) => true,
            (Some(expect), Some(actual)) => expect == actual,
        })
        .filter(|rule| match (&rule.supply_type, &supply_type) {
            (None, _) | (_, None) => true,
            (Some(expect), Some(actual)) => expect == actual,
        })
        .filter(|rule| match (&rule.online, &online) {
            (None, _) | (_, None) => true,
            (Some(expect), Some(actual)) => expect == actual,
        })
    {
        let mut fields = HashMap::new();
        fields.insert("name", name.clone());
        fields.insert("type", supply_type.clone());
        fields.insert("online", online.clone());

        maybe_exec(rule.message.exec.as_ref());
        task::spawn_blocking(move || {
            rule.message.notify(
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
