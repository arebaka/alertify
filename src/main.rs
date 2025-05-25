mod message;
mod config;
mod battery;
mod memory;
mod storage;
mod udev;
mod utils;

use anyhow::Result;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::spawn;

use crate::{
    config::get_config,
    battery::monitor_battery,
    memory::monitor_memory,
    storage::monitor_storage,
    udev::listen_udev,
};

#[tokio::main]
async fn main() -> Result<()> {
    let config = get_config()?;

    let sent = Arc::new(Mutex::new(HashSet::new()));
    let handles = vec![
        spawn(monitor_battery(config.battery.clone(), sent.clone())),
        spawn(monitor_memory(config.memory.clone(), sent.clone())),
        spawn(monitor_storage(config.storage.clone(), sent.clone())),
        // spawn(monitor_network(cfg.network.clone(), sent.clone())),
        // spawn(monitor_bluetooth(cfg.bluetooth.clone(), sent.clone())),
    ];

    listen_udev(config, sent.clone()).await.unwrap();
    for h in handles {
        let _ = h.await;
    }

    Ok(())
}
