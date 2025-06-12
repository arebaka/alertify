use anyhow::Result;
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}, time::Duration};
use sysinfo::System;
use tokio::{task, time::sleep};

use crate::{config::CPURule, utils::execute_command};

pub async fn monitor_cpu(rules: Vec<CPURule>, sent: Arc<Mutex<HashSet<String>>>) -> Result<()> {
    loop {
        let mut sys = System::new();
        sys.refresh_cpu_all();
        sys.refresh_cpu_usage();

        let used_percent = sys.global_cpu_usage();
        let left_percent = 100.0 - used_percent;

        let freqs = sys.cpus().iter().map(|cpu| cpu.frequency());
        let max_freq = freqs.clone().max().unwrap();
        let avg_freq = freqs.sum::<u64>() / sys.cpus().len() as u64;

        for rule in &rules {
            let should_notify = {
                let key = format!("cpu-{}", rule.level);
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
                fields.insert("max_freq",          max_freq.to_string());
                fields.insert("avg_freq",          avg_freq.to_string());
                fields.insert("used_percent_full", used_percent.to_string());
                fields.insert("used_percent",      (used_percent as u32).to_string());
                fields.insert("left_percent_full", left_percent.to_string());
                fields.insert("left_percent",      (left_percent as u32).to_string());

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
