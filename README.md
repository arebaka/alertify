# alertify
*uwu nya*

> **alertify** is a lightweight and efficient system notification tool written in Rust. It monitors system events like battery status, memory usage, device connections, and more, and sends desktop notifications to keep you informed in real time.

---

## Features

- Battery level monitoring with alerts
- Memory and storage usage notifications
- Device connection and disconnection events (via udev)
- Monitoring of power supply status changing
- Configurable via a TOML configuration file
- Built with modern Rust async ecosystem (Tokio, Zbus, etc.)

---

## Installation

### From source

You can build **alertify** from source using Cargo:

```bash
git clone https://github.com/arebaka/alertify.git
cd alertify
cargo build --release
