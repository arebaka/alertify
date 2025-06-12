use futures_util::stream::StreamExt;
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};
use zbus::{Connection, MessageStream, zvariant::OwnedValue};

use crate::config::Config;

pub async fn listen_network_prop_changes(_config: Config, _sent: Arc<Mutex<HashSet<String>>>) -> zbus::Result<()> {
    let conn = Connection::system().await?;
    let mut stream = MessageStream::from(&conn);

    while let Some(result) = stream.next().await {
        println!("{:?}", result);
        let message = match result {
            Ok(m) => m,
            _     => {
                continue;
            }
        };

        // Extract the member (signal name)
        let header = message.header();
        let member = match header.member() {
            Some(m) => m,
            _ => continue,
        };

        if member.as_str() != "PropertiesChanged" {
            continue;
        }

        // Deserialize the body of the message
        let body: (String, HashMap<String, OwnedValue>, Vec<String>) =
            message.body().deserialize()?;

        let (interface, changed_props, _invalidated) = body;

        if interface != "org.freedesktop.NetworkManager" {
            continue;
        }

        if let Some(state_val) = changed_props.get("State") {
            let state = state_val.downcast_ref::<u32>();
            match state {
                Ok(20) => println!("NM: disconnected"),
                Ok(70) => println!("NM: connected"),
                _      => ()
            }
        }
    }
    println!("2");

    Ok(())
}
