# alertify
*uwu nya*

> **alertify** is a lightweight and efficient system notification tool written in Rust. It monitors system events like battery status, memory usage, device connections, and more, and sends desktop notifications to keep you informed in real time.

![screenshot](https://github.com/user-attachments/assets/91692f46-f19c-441c-b709-77b9b2f6c4ab)

---

## Features

- Battery level monitoring with alerts
- CPU, memory and storage usage notifications
- Device connection and disconnection events (via udev)
- Monitoring of power supply status changing
- Configurable via a TOML configuration file
- Built with modern Rust async ecosystem (Tokio, Zbus, etc.)

---

## Installation

### From AUR

```bash
yay -S alertify
```

### From source

You can build **alertify** from source using Cargo:

```bash
git clone https://github.com/arebaka/alertify.git
cd alertify
cargo build --release
```
## Configuration

`alertify` is configured via a single `config.toml` file. It defines a list of notification rules for system events like battery level, memory usage, disk space, device connections, and power supply status.

The file is loaded from:

1. `$XDG_CONFIG_HOME/alertify/config.toml`
2. `~/.config/alertify/config.toml`

### Example configuration

```toml
[[battery]]
level = 20
urgency = "critical"
appname = ""
summary = "Low Battery"
body = "Battery level dropped to {left_percent}%!"
icon = "battery-caution-symbolic"
hints = ["transient", "category:battery", "string:x-dunst-stack-tag:battery.low"]

[[battery]]
level = 5
urgency = "critical"
appname = ""
summary = "Critical Battery Level"
body = "Battery level dropped to {left_percent}%!"
icon = "battery-empty-symbolic"
timeout = 0
hints = ["transient", "category:battery", "string:x-dunst-stack-tag:battery.low"]

[[cpu]]
level = 90.0
urgency = "normal"
appname = ""
summary = "CPU usage is at {used_percent}%!"
body = "Maximum clock frequency: {max_freq} KHz, average: {avg_freq} KHz."
icon = "dialog-warning-symbolic"
hints = ["transient", "category:cpu", "string:x-dunst-stack-tag:cpu.high"]

[[memory]]
level = 90.0
urgency = "normal"
appname = ""
summary = "RAM usage is at {used_percent}%!"
body = "Total available: {total}, remaining: {left}."
icon = "dialog-warning-symbolic"
hints = ["transient", "category:memory", "string:x-dunst-stack-tag:memory.high"]

[[storage]]
level = 95.0
urgency = "normal"
appname = ""
summary = "Less than {left_percent}% disk space available!"
body = "Device {name} total: {total}, left: {left}."
icon = "drive-harddisk-symbolic"
hints = ["category:storage", "string:x-dunst-stack-tag:storage.high"]

[[device]]
action = "add"
subsystem = "block"
devtype = "disk"
urgency = "low"
appname = ""
summary = "Device Connected"
body = "A new USB device was connected: {devnode}"
icon = "media-flash-symbolic"
hints = ["transient", "category:block", "string:x-dunst-stack-tag:block.{devnum}"]

[[device]]
action = "remove"
subsystem = "block"
devtype = "disk"
urgency = "low"
appname = ""
summary = "Device Disconnected"
body = "USB device was disconnected: {devnode}"
icon = "media-flash-symbolic"
hints = ["transient", "category:block", "string:x-dunst-stack-tag:block.{devnum}"]

[[power_supply]]
type = "Mains"
online = "1"
urgency = "low"
appname = ""
summary = "Power cable connected"
body = ""
icon = "ac-adapter-symbolic"
hints = ["transient", "category:power-supply", "string:x-dunst-stack-tag:power-supply"]

[[power_supply]]
type = "Mains"
online = "0"
urgency = "low"
appname = ""
summary = "Power cable disconnected"
body = ""
icon = "ac-adapter-symbolic"
hints = ["transient", "category:power-supply", "string:x-dunst-stack-tag:power-supply"]
```

### Supported sections

Each section type corresponds to a system resource or event and accepts multiple entries:

- `[[battery]]`: Notifications for low battery levels
- `[[cpu]]`: CPU usage alerts
- `[[memory]]`: RAM usage alerts
- `[[storage]]`: Low disk space warnings
- `[[device]]`: USB or other device events (via udev)
- `[[power_supply]]`: AC adapter plugged/unplugged events

### Common fields

| Field         | Type    | Default value      | Description                                                          |
| ------------- | ------- | ------------------ | -------------------------------------------------------------------- |
| `urgency`     | String  | `"normal"`         | `"low"`, `"normal"`, or `"critical"` (affects notification priority) |
| `appname`     | String  | `"*Section name*"` | Displayed as the notification source name                            |
| `summary`     | String  | `""`               | Main title of the notification; supports placeholders                |
| `body`        | String  | `""`               | Optional text body; also supports placeholders                       |
| `icon`        | String  | `""`               | Icon name (freedesktop-compliant)                                    |
| `timeout`     | Integer | None               | Time in milliseconds to show the notification (`0` = persistent)     |
| `hints`       | List    | `[]`               | List of notification hints (D-Bus extras)                            |

### Section fields

| Field         | Type    | Sections                                              | Default value                                      | Description                                                             |
| ------------- | ------- | ----------------------------------------------------- | -------------------------------------------------- |------------------------------------------------------------------------ |
| `level`       | Number  | `[[battery]]`, `[[cpu]]`, `[[memory]]`, `[[storage]]` | `20` (battery), `90` (cpu, memory), `95` (storage) | Threshold value (e.g. percent for battery/memory/storage usage)         |
| `action`      | String  | `[[device]]`                                          | `"add"`                                            | Udev device event type: `add`, `remove`, `bind`, `unbind`, `change`     |
| `initialized` | Boolean | `[[device]]`                                          | None                                               | Whether the device is already initialized when matching                 |
| `subsystem`   | String  | `[[device]]`                                          | None                                               | Device subsystem to match, e.g. `"usb"`, `"block"`, `"net"`             |
| `sysname`     | String  | `[[device]]`                                          | None                                               | Match specific system name (e.g. `"sda1"`)                              |
| `sysnum`      | Integer | `[[device]]`                                          | None                                               | Match specific system number if needed                                  |
| `devtype`     | String  | `[[device]]`                                          | None                                               | Match the device type, e.g. `"usb_device"`, `"partition"`               |
| `driver`      | String  | `[[device]]`                                          | None                                               | Match the kernel driver, e.g. `"usb-storage"`.                          |
| `name`        | String  | `[[power_supply]]`                                    | None                                               | Power supply device name, e.g. `"AC"`, `"BAT0"`                         |
| `supply_type` | String  | `[[power_supply]]`                                    | None                                               | Filter for type of power supply, e.g. `"Mains"`, `"Battery"`            |
| `online`      | String  | `[[power_supply]]`                                    | None                                               | `"1"` when connected, `"0"` when disconnected                           |

### Supported Placeholders

You can use dynamic placeholders in `appname`, `summary` and `body` fields:

| Field                 | Sections                                              | Description                                           |
| --------------------- | ----------------------------------------------------- | ----------------------------------------------------- |
| `{level}`             | `[[battery]]`, `[[cpu]]`, `[[memory]]`, `[[storage]]` | Current threshold level (percentage or numeric value) |
| `{max_freq}`          | `[[cpu]]`                                             | Maximum clock frequency in KHz of one core            |
| `{avg_freq}`          | `[[cpu]]`                                             | average clokc frequency in KHz of all cpu cores       |
| `{left_percent_full}` | `[[battery]]`, `[[cpu]]`, `[[memory]]`, `[[storage]]` | Remaining percent with fractional precision           |
| `{left_percent}`      | `[[battery]]`, `[[cpu]]`, `[[memory]]`, `[[storage]]` | Remaining percent rounded to integer                  |
| `{used_percent_full}` | `[[battery]]`, `[[cpu]]`, `[[memory]]`, `[[storage]]` | Used percent with fractional precision                |
| `{used_percent}`      | `[[battery]]`, `[[cpu]]`, `[[memory]]`, `[[storage]]` | Used percent rounded to integer                       |
| `{total_bytes}`       | `[[memory]]`, `[[storage]]`                           | Total memory or storage size in bytes                 |
| `{total}`             | `[[memory]]`, `[[storage]]`                           | Total size in human-readable format, e.g. `2.5 GB`    |
| `{used_bytes}`        | `[[memory]]`, `[[storage]]`                           | Used memory or storage in bytes                       |
| `{used}`              | `[[memory]]`, `[[storage]]`                           | Used memory or storage in human-readable format       |
| `{left_bytes}`        | `[[memory]]`, `[[storage]]`                           | Remaining memory or storage in bytes                  |
| `{left}`              | `[[memory]]`, `[[storage]]`                           | Remaining memory or storage in human-readable format  |
| `{kind}`              | `[[storage]]`                                         | Storage kind, e.g. `disk`, `partition`                |
| `{name}`              | `[[storage]]`                                         | Device name, e.g. `sda1`                              |
| `{fs}`                | `[[storage]]`                                         | Filesystem type, e.g. `ext4`, `btrfs`                 |
| `{mount}`             | `[[storage]]`                                         | Mount point, e.g. `/home`                             |
| `{subsystem}`         | `[[device]] `                                         | Udev subsystem, e.g. `usb`, `net`, `block`            |
| `{sysname}`           | `[[device]] `                                         | System name, e.g. `sda`, `event4`                     |
| `{sysnum}`            | `[[device]] `                                         | System number                                         |
| `{devtype}`           | `[[device]] `                                         | Device type, e.g. `usb_device`, `partition`           |
| `{driver}`            | `[[device]] `                                         | Kernel driver name, e.g. `usb-storage`                |
| `{seq_num}`           | `[[device]] `                                         | Kernel event sequence number                          |
| `{syspath}`           | `[[device]] `                                         | Full sysfs path of the device                         |
| `{devpath}`           | `[[device]] `                                         | Udev device path, e.g. `/devices/.../usb1`            |
| `{devnode}`           | `[[device]] `                                         | Device node path, e.g. `/dev/sda`                     |
