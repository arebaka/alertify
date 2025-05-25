use anyhow::Result;
use humansize::{format_size, DECIMAL};
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}, time::Duration};
use sysinfo::{DiskKind, Disks};
use tokio::{task, time::sleep};

use crate::{config::StorageCase, utils::maybe_exec};

pub async fn monitor_storage(cfg: Vec<StorageCase>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
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
                    fields.insert("total", format_size(total, DECIMAL));
                    fields.insert("used_bytes", used.to_string());
                    fields.insert("used", format_size(used, DECIMAL));
                    fields.insert("used_percent_full", used_percent.to_string());
                    fields.insert("value_percent", (used_percent as u32).to_string());
                    fields.insert("left_bytes", left.to_string());
                    fields.insert("left", format_size(left, DECIMAL));
                    fields.insert("left_percent_full", left_percent.to_string());
                    fields.insert("left_percent", (left_percent as u32).to_string());

                    let case_clone = case.clone();
                    maybe_exec(case_clone.message.exec.as_ref());
                    task::spawn_blocking(move || {
                        case_clone.message.notify(&fields);
                    })
                    .await?;
                }
            }
        }

        sleep(Duration::from_secs(60)).await;
    }
}
