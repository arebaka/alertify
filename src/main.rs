mod message;
mod config;
mod battery;
mod cpu;
mod memory;
mod storage;
mod udev;
mod utils;

use anyhow::Result;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::spawn;
use log::{error, info};

use crate::{
    config::get_config,
    battery::monitor_battery,
    cpu::monitor_cpu,
    memory::monitor_memory,
    storage::monitor_storage,
    udev::listen_udev,
};

type SharedNotificationSet = Arc<Mutex<HashSet<String>>>;
type TaskHandles = Vec<tokio::task::JoinHandle<()>>;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    info!("Starting monitor");

    let config = get_config()?;
    let sent = Arc::new(Mutex::new(HashSet::new()));

    let tasks = start_tasks(config.clone(), sent.clone());

    // blocking
    let udev_result = listen_udev(config, sent).await;
    if let Err(e) = udev_result {
        error!("UDev listener failed: {}", e);
        return Err(e);
    }

    wait_for_tasks_completion(tasks).await;

    info!("Monitor shutdown complete");
    Ok(())
}


fn start_tasks(config: crate::config::Config, sent: SharedNotificationSet) -> TaskHandles {
    let battery_config = config.battery.clone();
    let cpu_config = config.cpu.clone();
    let memory_config = config.memory.clone();
    let storage_config = config.storage.clone();

    let sent1 = sent.clone();
    let sent2 = sent.clone();
    let sent3 = sent.clone();
    let sent4 = sent.clone();

    vec![
        spawn(async move {
            if let Err(e) = monitor_battery(battery_config, sent1).await {
                error!("Battery monitor failed: {}", e);
            }
        }),
        spawn(async move {
            if let Err(e) = monitor_cpu(cpu_config, sent2).await {
                error!("CPU monitor failed: {}", e);
            }
        }),
        spawn(async move {
            if let Err(e) = monitor_memory(memory_config, sent3).await {
                error!("Memory monitor failed: {}", e);
            }
        }),
        spawn(async move {
            if let Err(e) = monitor_storage(storage_config, sent4).await {
                error!("Storage monitor failed: {}", e);
            }
        }),
    ]
}

async fn wait_for_tasks_completion(handles: TaskHandles) {
    for handle in handles {
        if let Err(e) = handle.await {
            error!("Task failed to complete: {}", e);
        }
    }
}
