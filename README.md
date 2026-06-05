# Omni Monitor

Standalone hardware monitor for the SteelSeries Nova Pro Wireless GameDAC (Omni) OLED display.

## Display layout
CPU  XX%   XX   XX%   - load / temp / RAM%
GPU  XX%   XX   XX%   - load / temp / VRAM%
Omni XX%   G703 XX%   - headset / G703 mouse battery

## Requirements
- Windows 10/11
- Argus Monitor running
- Nova Pro Wireless GameDAC via USB

## Usage
Run omni_monitor.exe (no console window).
Config: omni_monitor.conf next to the exe (brightness=100).

## Build
cargo build --release --package omni_monitor
ggoled_lib is a patched fork of JerwuQu/ggoled with Omni GameDAC support.

## License
MIT
