use anyhow::Result;
use humansize::{format_size, DECIMAL};
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}, time::Duration};
use sysinfo::System;
use tokio::{task, time::sleep};

use crate::{config::MemoryCase, utils::maybe_exec};

pub async fn monitor_memory(cfg: Vec<MemoryCase>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
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
                fields.insert("total", format_size(total, DECIMAL));
                fields.insert("used_bytes", used.to_string());
                fields.insert("used", format_size(used, DECIMAL));
                fields.insert("used_percent_full", used_percent.to_string());
                fields.insert("used_percent", (used_percent as u32).to_string());
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

        sleep(Duration::from_secs(10)).await;
    }
}
