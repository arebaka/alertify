use notify_rust::Urgency;
use tokio::process::Command;
use log::debug;
use anyhow::{Context, Result};

pub fn parse_urgency(s: &str) -> Urgency {
    match s.to_lowercase().as_str() {
        "low"      => Urgency::Low,
        "normal"   => Urgency::Normal,
        "critical" => Urgency::Critical,
        _ => Urgency::Normal,
    }
}

pub fn execute_command(command: Option<&String>) -> Result<()> {
    let Some(cmd) = command else {
        return Ok(());
    };

    if cmd.trim().is_empty() {
        return Ok(());
    }

    debug!("Executing command: {}", cmd);

    Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .spawn()
        .with_context(|| format!("Failed to spawn command: {}", cmd))?;

    Ok(())
}
