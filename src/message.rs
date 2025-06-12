use serde::Deserialize;
use notify_rust::{Hint, Notification, Timeout};
use regex::{Captures, Regex};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use anyhow::{Context, Result};
use log::{error, debug};

use crate::utils::{parse_urgency, execute_command};

static TEMPLATE_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_template_regex() -> &'static Regex {
    TEMPLATE_REGEX.get_or_init(|| {
        Regex::new(r"\{([a-zA-Z0-9_]+)\}")
            .expect("Failed to compile template regex")
    })
}

fn default_urgency() -> String {
    "normal".to_string()
}

fn default_appname() -> String {
    "alertify".to_string()
}

#[derive(Default, Debug, Deserialize, Clone)]
pub struct Message {
   #[serde(default = "default_urgency")]
    pub urgency: String,
    #[serde(default = "default_appname")]
    pub appname: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    pub timeout: Option<u32>,
    #[serde(default)]
    pub hints: HashSet<MyHint>,
    #[serde(default)]
    pub exec: Option<String>,
}

impl Message {
    fn render_template(template: &str, fields: &HashMap<&str, String>) -> String {
        if template.is_empty() {
            return String::new();
        }

        let regex = get_template_regex();
        regex.replace_all(template, |caps: &Captures| {
            let key = &caps[1];
            match fields.get(key) {
                Some(value) => value.clone(),
                None => caps[0].to_string()
            }
        })
        .to_string()
    }

    pub fn notify(&self, fields: &HashMap<&str, String>) -> Result<()> {
        let urgency = parse_urgency(&self.urgency);
        let mut notification = Notification::new();

        notification.urgency(urgency);

        let rendered_appname = Self::render_template(&self.appname, fields);
        notification.appname(&rendered_appname);

        if let Some(ref summary) = self.summary {
            notification.summary(&Self::render_template(summary, fields));
        }

        if let Some(ref body) = self.body {
            notification.body(&Self::render_template(body, fields));
        }

        if let Some(ref icon) = self.icon {
            notification.icon(icon);
        }

        if let Some(timeout_ms) = self.timeout {
            notification.timeout(Timeout::Milliseconds(timeout_ms));
        }

        for hint in &self.hints {
            let rendered = hint.render(fields);
            notification.hint(rendered.into());
        }

        notification.show()
            .with_context(|| "Failed to show notification")?;

        debug!("Notification sent: {}", self.appname);

        if let Some(ref command) = self.exec {
            let rendered = Self::render_template(command, fields);
            let _ = execute_command(Some(&rendered));
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct MyHint(String);

impl MyHint {
    pub fn new(hint: String) -> Self {
        Self(hint)
    }

    fn render(&self, fields: &HashMap<&str, String>) -> Self {
        Self(Message::render_template(&self.0.clone(), fields))
    }

    fn parse_components(&self) -> HintComponents {
        let parts: Vec<&str> = self.0.rsplitn(3, ':').collect();

        match parts.as_slice() {
            [value, key, hint_type] => HintComponents {
                hint_type: Some(hint_type),
                key,
                value,
            },
            [value, key] => HintComponents {
                hint_type: None,
                key,
                value,
            },
            [key] => HintComponents {
                hint_type: None,
                key,
                value: "",
            },
            _ => HintComponents {
                hint_type: None,
                key: &self.0,
                value: "",
            },
        }
    }
}

#[derive(Debug)]
struct HintComponents<'a> {
    hint_type: Option<&'a str>,
    key: &'a str,
    value: &'a str,
}

impl From<MyHint> for Hint {
    fn from(hint: MyHint) -> Hint {
        let components = hint.parse_components();

        // Try to create a typed hint first
        if let Some(hint_type) = components.hint_type {
            match hint_type {
                "bool" => {
                    if let Ok(bool_val) = components.value.parse::<bool>() {
                        return Hint::from_key_val(components.key, &bool_val.to_string())
                            .unwrap_or_else(|_| Hint::Custom(components.key.to_string(), components.value.to_string()));
                    }
                }
                "int" => {
                    if let Ok(int_val) = components.value.parse::<i32>() {
                        return Hint::from_key_val(components.key, &int_val.to_string())
                            .unwrap_or_else(|_| Hint::Custom(components.key.to_string(), components.value.to_string()));
                    }
                }
                "double" => {
                    if let Ok(double_val) = components.value.parse::<f64>() {
                        return Hint::from_key_val(components.key, &double_val.to_string())
                            .unwrap_or_else(|_| Hint::Custom(components.key.to_string(), components.value.to_string()));
                    }
                }
                "string" => {
                    return Hint::from_key_val(components.key, components.value)
                        .unwrap_or_else(|_| Hint::Custom(components.key.to_string(), components.value.to_string()));
                }
                _ => {
                    error!("Unknown hint type: {}", hint_type);
                }
            }
        }

        // Fallback to string hint or custom hint
        Hint::from_key_val(components.key, components.value)
            .unwrap_or_else(|_| Hint::Custom(components.key.to_string(), components.value.to_string()))
    }
}
