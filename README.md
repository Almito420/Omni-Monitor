<div align="center">

# Omni Monitor

**Standalone OLED hardware monitor for the SteelSeries Nova Pro Wireless GameDAC**

Real-time CPU, GPU, RAM, VRAM and battery stats on the GameDAC's 128x64 OLED display.
No SteelSeries GG. No Python. No dependencies. Single `.exe`, ~550 KB.

</div>

---

## What it looks like

```
+----------------------------------+
|  CPU   2%   54'   67%           |   load / temperature / RAM usage
|  GPU   8%   43'    9%           |   load / temperature / VRAM usage
|  Omni 84%   G703  71%           |   headset battery / mouse battery
+----------------------------------+
```

Every value updates in real time. The display blanks automatically when the monitor sleeps.

---

## Data sources

| Row | Value | Source |
|-----|-------|--------|
| CPU | Load % | Argus Monitor shared memory |
| CPU | Temperature | Argus Monitor shared memory |
| CPU | RAM % | Windows `GlobalMemoryStatusEx` |
| GPU | Load % | Argus Monitor shared memory |
| GPU | Temperature | Argus Monitor shared memory |
| GPU | VRAM % | Windows DXGI (total) + PDH counter (used) |
| Bottom | Omni headset % | USB HID, usage page `0xFF00` |
| Bottom | G703 mouse % | HID++ 2.0 feature `0x1001` (Battery Voltage), voltage-to-% LUT |

---

## Features

- Zero runtime dependencies -- single `.exe`, copy and run
- Silent background process -- no console window
- Auto-start support via Windows Startup folder or Task Scheduler
- OLED burn-in prevention -- 9-position pixel shift (90 s/step), ported from [ggoled](https://github.com/JerwuQu/ggoled)
- Monitor sleep detection -- display blanks via `RegisterPowerSettingNotification`
- Configurable brightness (0-100%) via `omni_monitor.conf`
- Auto-reconnect after USB disconnect
- ~1% CPU usage, ~5 MB RAM footprint

---

## Requirements

| Requirement | Notes |
|-------------|-------|
| Windows 10 / 11 (x64) | |
| [Argus Monitor](https://www.argusmonitor.com/) | Must be running. Enable **Shared Memory Support** in Settings |
| Nova Pro Wireless GameDAC | Connected via USB |
| Logitech G703 *(optional)* | Battery shown automatically if connected |

---

## Quick start

### 1. Download

Grab **`omni_monitor.exe`** from the [latest release](../../releases/latest) or directly from this repository.

### 2. Run

Double-click `omni_monitor.exe`. It starts silently -- no window appears.
On first run, `omni_monitor.conf` is created next to the exe.

### 3. Auto-start (optional)

**Option A -- Startup folder:**
Copy `omni_monitor_start.vbs` (included) next to the exe, then place a shortcut to it in:
```
%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup
```

**Option B -- Task Scheduler:**
- Create Task
- Trigger: At log on
- Action: Start a program -> `omni_monitor.exe`
- Conditions: uncheck "Start only on AC power"

---

## Configuration

`omni_monitor.conf` is created automatically next to the exe on first run:

```ini
# Omni Monitor config
# brightness: 0-100 (percent)
brightness=100
```

Edit the value, save, and restart the exe. Brightness is mapped to the device's 1-10 internal scale.

---

## Files in this repository

| File | Description |
|------|-------------|
| `omni_monitor.exe` | Ready-to-run Windows executable |
| `omni_monitor.conf` | Config template (brightness) |
| `omni_monitor_start.vbs` | Silent launcher (no console flash) |
| `omni_monitor/` | Rust source code |
| `ggoled_lib/` | Patched fork of [JerwuQu/ggoled](https://github.com/JerwuQu/ggoled) with Omni GameDAC support |
| `ggoled_draw/` | Drawing utilities + PixelOperator font |

---

## Building from source

Requires [Rust stable](https://rustup.rs).

```bash
git clone https://github.com/Almito420/Omni-Monitor.git
cd Omni-Monitor
cargo build --release --package omni_monitor
# output: target/release/omni_monitor.exe
```

### Project structure

```
Omni-Monitor/
|-- omni_monitor/src/
|   |-- main.rs       threads, render loop, reconnect
|   |-- argus.rs      Argus Monitor shared memory reader
|   |-- vram.rs       VRAM % via DXGI + PDH
|   |-- sysinfo.rs    RAM % via GlobalMemoryStatusEx
|   |-- g703.rs       G703 HID++ 2.0 battery reader
|   |-- monitor.rs    Win32 power notification
|   |-- render.rs     PixelOperator 16px bitmap renderer
|   +-- config.rs     omni_monitor.conf parser
|-- ggoled_lib/       patched ggoled library
+-- ggoled_draw/      font + drawing utilities
```

---

## Protocol notes

The Nova Pro Omni GameDAC protocol was reverse-engineered from USB pcap captures.

| Field | Value |
|-------|-------|
| USB VID / PID | `0x1038` / `0x2290` |
| OLED interface | usage page `0xFFC0` |
| Battery interface | usage page `0xFF00` |
| Report ID | `0x01` |
| OLED command | `0x93` |
| Report size | 1036 bytes (1 ID + 7 header + 1028 bitmap) |
| Bitmap format | Column-major, 1 bpp, LSB-first |
| Left half header | `93 00 00 40 40 00 00` |
| Right half header | `93 40 00 40 40 00 00` |

`ggoled_lib` adds `DeviceModel::NovaProOmni` to the existing [ggoled](https://github.com/JerwuQu/ggoled) device detection, reusing the column-major bitmap encoder with an adjusted report frame.

---

## License

MIT -- see [LICENSE](LICENSE)

`ggoled_lib` and `ggoled_draw` are derived from [JerwuQu/ggoled](https://github.com/JerwuQu/ggoled).
PixelOperator font by Jayvee Enaguas -- free for personal and commercial use.
