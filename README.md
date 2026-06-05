# Omni Monitor

A lightweight, standalone OLED hardware monitor for the **SteelSeries Nova Pro Wireless GameDAC**.
Displays real-time CPU, GPU, RAM, VRAM and battery data on the GameDAC's 128x64 OLED screen — no SteelSeries GG required.

Built in Rust. Single executable, ~550 KB, zero console window, minimal CPU usage.

---

## Display layout

```
+----------------------------------+
| CPU   2%    54    67%            |  <- load / temp / RAM%
| GPU   8%    43     9%            |  <- load / temp / VRAM%
| Omni 84%   G703  71%             |  <- headset battery / mouse battery
+----------------------------------+
```

| Position | Source | Description |
|----------|--------|-------------|
| CPU load % | Argus Monitor | Total CPU utilisation |
| CPU temp | Argus Monitor | Package / die temperature |
| RAM % | Windows API | Physical memory usage |
| GPU load % | Argus Monitor | GPU core utilisation |
| GPU temp | Argus Monitor | GPU temperature |
| VRAM % | DXGI + PDH | Dedicated VRAM used / total |
| Omni % | USB HID | Nova Pro Wireless headset battery |
| G703 % | HID++ 2.0 | Logitech G703 mouse battery |

---

## Features

- **Zero runtime dependencies** — single `.exe`, no installers, no runtimes needed
- **Argus Monitor integration** — CPU/GPU load and temperature via shared memory
- **RAM usage** via `GlobalMemoryStatusEx` (always accurate, no third-party needed)
- **VRAM usage %** — dedicated VRAM used vs. total via DXGI + Windows PDH counters
- **Headset battery** — Nova Pro Wireless battery level via native USB HID
- **G703 mouse battery** — Logitech G703 LIGHTSPEED via HID++ 2.0 (feature 0x1001, voltage to % LUT from LGSTrayBattery)
- **OLED burn-in prevention** — 9-position +-1px pixel shift (90 s/step), ported from [ggoled](https://github.com/JerwuQu/ggoled)
- **Monitor sleep detection** — display blanks automatically when Windows turns off the monitor
- **Brightness control** via `omni_monitor.conf` (0-100 %, applied at startup)
- **Auto-reconnect** — survives USB disconnects and device resets

---

## Requirements

| Requirement | Notes |
|-------------|-------|
| Windows 10 / 11 | x64 |
| [Argus Monitor](https://www.argusmonitor.com/) | Must be running. Enable: Settings -> Shared Memory Support |
| Nova Pro Wireless GameDAC | Connected via USB (VID 0x1038, PID 0x2290) |
| Logitech G703 *(optional)* | Battery shown if connected |

---

## Installation

1. Download **`omni_monitor.exe`** from the [latest release](../../releases/latest)
2. Place it in any folder
3. Run it — starts silently, no console window
4. `omni_monitor.conf` is created automatically next to the exe

### Auto-start at login

Place a shortcut to `omni_monitor.exe` in:

```
%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup
```

Or use **Task Scheduler**: Create Task -> Trigger: At log on -> Action: path to exe.

---

## Configuration

`omni_monitor.conf` is created on first run:

```ini
# Omni Monitor config
# brightness: 0-100 (percent)
brightness=100
```

Edit and restart the exe to apply.

---

## Building from source

**Requires:** Rust stable ([rustup.rs](https://rustup.rs))

```bash
git clone https://github.com/Almito420/Omni-Monitor.git
cd Omni-Monitor
cargo build --release --package omni_monitor
```

Binary: `target/release/omni_monitor.exe`

### Workspace structure

```
Omni-Monitor/
+-- Cargo.toml              # workspace
+-- ggoled_lib/             # patched fork of JerwuQu/ggoled
|   +-- src/lib.rs          #   added: Nova Pro Omni GameDAC (PID 0x2290) support
+-- ggoled_draw/            # drawing utilities + PixelOperator font
+-- omni_monitor/           # hardware monitor binary
    +-- src/
        +-- main.rs         # threads, render loop, monitor sleep
        +-- argus.rs        # Argus Monitor shared memory reader
        +-- vram.rs         # VRAM % via DXGI + PDH
        +-- sysinfo.rs      # RAM % via GlobalMemoryStatusEx
        +-- g703.rs         # Logitech G703 HID++ 2.0 battery reader
        +-- monitor.rs      # Win32 power notification (monitor sleep)
        +-- render.rs       # bitmap renderer (PixelOperator 16px font)
        +-- config.rs       # omni_monitor.conf parser
```

---

## Protocol notes

The Nova Pro Omni GameDAC was reverse-engineered from USB pcap captures (Wireshark + USBPcap).

| Detail | Value |
|--------|-------|
| VID / PID | 0x1038 / 0x2290 |
| OLED interface | usage_page = 0xFFC0 |
| Battery interface | usage_page = 0xFF00 |
| Report ID | 0x01 |
| OLED command | 0x93 (write bitmap) |
| Report size | 1036 bytes (1 report ID + 7 header + 1028 bitmap) |
| Bitmap encoding | Column-major, 1 bpp, LSB-first |
| Left half header | 93 00 00 40 40 00 00 |
| Right half header | 93 40 00 40 40 00 00 |

`ggoled_lib` is a patched fork of [JerwuQu/ggoled](https://github.com/JerwuQu/ggoled) with added Omni GameDAC support. The patch adds `DeviceModel::NovaProOmni` and reuses the existing column-major bitmap machinery with an adjusted report format.

---

## License

MIT — see [LICENSE](LICENSE)

`ggoled_lib` and `ggoled_draw` are derived from [JerwuQu/ggoled](https://github.com/JerwuQu/ggoled).
PixelOperator font by Jayvee Enaguas — free for personal & commercial use.
