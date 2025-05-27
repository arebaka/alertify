use serde::Deserialize;
use notify_rust::{Hint, Notification, Timeout};
use regex::{Captures, Regex};
use std::collections::{HashMap, HashSet};

use crate::{utils::parse_urgency};

#[derive(Default, Debug, Deserialize, Clone)]
pub struct Message {
    pub urgency: String,
    pub appname: String,
    pub summary: String,
    pub body: String,
    pub icon: String,
    pub timeout: Option<u32>,
    pub hints: HashSet<MyHint>,
    #[serde(default)]
    pub exec: Option<String>,
}

impl Message {
    fn render(template: String, fields: &HashMap<&str, String>) -> String {
        let re = Regex::new(r"\{([a-zA-Z0-9_]+)\}").unwrap();
        re.replace_all(&template, |caps: &Captures| {
            let key = &caps[1];
            fields
                .get(key)
                .cloned()
                .unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned()
    }

    pub fn notify(&self, fields: &HashMap<&str, String>) {
        let urgency = parse_urgency(&self.urgency);
        let mut notification = Notification::new();

        notification
            .urgency(urgency)
            .appname(&Self::render(self.appname.clone(), fields))
            .summary(&Self::render(self.summary.clone(), fields))
            .body(&Self::render(self.body.clone(), fields))
            .icon(&self.icon);
        if let Some(timeout) = self.timeout {
            notification.timeout(Timeout::Milliseconds(timeout));
        }

        for hint in &self.hints {
            let rendered = MyHint(Self::render(hint.0.clone(), fields));
            notification.hint(rendered.into());
        }

        let _ = notification.show();
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct MyHint(String);

impl From<MyHint> for Hint {
    fn from(hint: MyHint) -> Hint {
        let s = hint.0.as_str();
        let parts: Vec<&str> = s.rsplitn(3, ':').collect();

        let (key, value) = match *parts.as_slice() {
            // bool:transient:true
            [val, key, "bool"] => (key, val),
            // int:volume:100
            [val, key, "int"] => (key, val),
            // double:progress:0.75
            [val, key, "double"] => (key, val),
            // string:x-dunst-stack-tag:battery.low
            [val, key, "string"] => (key, val),
            // fallback: no type, just key:value
            [val, key] => (key, val),
            // just key
            [key] => (key, ""),
            _ => (s, ""),
        };

        Hint::from_key_val(key, value).unwrap_or(Hint::Custom(key.to_string(), value.to_string()))
    }
}
