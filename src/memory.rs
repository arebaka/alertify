use anyhow::Result;
use humansize::{format_size, DECIMAL};
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}, time::Duration};
use sysinfo::System;
use tokio::{task, time::sleep};

use crate::{config::MemoryRule, utils::execute_command};

pub async fn monitor_memory(rules: Vec<MemoryRule>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
    loop {
        let mut sys = System::new();
        sys.refresh_memory();

        let total        = sys.total_memory();
        let used         = sys.used_memory();
        let free         = sys.free_memory();
        let used_percent = used as f32 / total as f32 * 100.0;
        let free_percent = 100.0 - used_percent;

        for rule in &rules {
            let should_notify = {
                let key = format!("memory-{}", rule.level);
                let mut sent_guard = sent.lock().unwrap();

                if used_percent >= rule.level {
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
                fields.insert("level",             rule.level.to_string());
                fields.insert("total_bytes",       total.to_string());
                fields.insert("total",             format_size(total, DECIMAL));
                fields.insert("used_bytes",        used.to_string());
                fields.insert("used",              format_size(used, DECIMAL));
                fields.insert("used_percent_full", used_percent.to_string());
                fields.insert("used_percent",      (used_percent as u32).to_string());
                fields.insert("left_bytes",        free.to_string());
                fields.insert("left",              format_size(free, DECIMAL));
                fields.insert("left_percent_full", free_percent.to_string());
                fields.insert("left_percent",      (free_percent as u32).to_string());

                let rule_clone = rule.clone();
                let _ = execute_command(rule_clone.message.exec.as_ref());
                task::spawn_blocking(move || {
                    let _ = rule_clone.message.notify(&fields);
                })
                .await?;
            }
        }

        sleep(Duration::from_secs(10)).await;
    }
}
