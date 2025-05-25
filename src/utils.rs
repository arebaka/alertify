use notify_rust::Urgency;
use tokio::process::Command;

pub fn parse_urgency(s: &str) -> Urgency {
    match s {
        "low" => Urgency::Low,
        "normal" => Urgency::Normal,
        "critical" => Urgency::Critical,
        _ => Urgency::Normal,
    }
}

pub fn maybe_exec(exec: Option<&String>) {
    if let Some(cmdline) = exec {
        let _ = Command::new("sh")
            .arg("-c")
            .arg(cmdline)
            .spawn();
    }
}
