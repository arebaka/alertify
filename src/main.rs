mod message;
mod config;
mod battery;
mod memory;
mod storage;
mod udev;
mod utils;

use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::spawn;

use crate::{
	config::Config,
	battery::monitor_battery,
	memory::monitor_memory,
	storage::monitor_storage,
	udev::listen_udev,
};

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = Path::new("config.toml");
    let config_raw = fs::read_to_string(config_path)?;
    let cfg: Config = toml::from_str(&config_raw)?;

    let sent = Arc::new(Mutex::new(HashSet::new()));
    let handles = vec![
        spawn(monitor_battery(cfg.battery.clone(), sent.clone())),
        spawn(monitor_memory(cfg.memory.clone(), sent.clone())),
        spawn(monitor_storage(cfg.storage.clone(), sent.clone())),
        // spawn(monitor_network(cfg.network.clone(), sent.clone())),
        // spawn(monitor_bluetooth(cfg.bluetooth.clone(), sent.clone())),
    ];
    listen_udev(cfg, sent.clone()).await.unwrap();
    for h in handles {
        let _ = h.await;
    }

    Ok(())
}
