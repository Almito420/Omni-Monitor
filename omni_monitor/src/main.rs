#![windows_subsystem = "windows"]

mod argus;
mod config;
mod g703;
mod monitor;
mod render;
mod sysinfo;
mod vram;

use argus::ArgusData;
use ggoled_lib::{Bitmap, Device};
use hidapi::HidApi;
use render::Renderer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const VID: u16 = 0x1038;
const PID: u16 = 0x2290;
const BATT_USAGE_PAGE: u16 = 0xFF00;

const SHIFTS: [(isize, isize); 9] =
    [(0,0),(0,-1),(1,-1),(1,0),(1,1),(0,1),(-1,1),(-1,0),(-1,-1)];
const SHIFT_EVERY: usize = 2727;

#[derive(Default, Clone, Copy)]
struct State {
    headset_pct: Option<u8>,
    mouse_pct:   Option<u8>,
    ram_pct:     Option<u32>,
    vram_pct:    Option<u32>,
    argus:       ArgusData,
}

fn main() -> anyhow::Result<()> {
    let state     = Arc::new(Mutex::new(State::default()));
    let screen_on = Arc::new(AtomicBool::new(true));

    // ── Argus poll (every 2 s) ────────────────────────────────────────────
    {
        let state     = Arc::clone(&state);
        let screen_on = Arc::clone(&screen_on);
        thread::spawn(move || loop {
            if screen_on.load(Ordering::Relaxed) {
                state.lock().unwrap().argus = argus::read();
            }
            thread::sleep(Duration::from_secs(2));
        });
    }

    // ── RAM + VRAM (every 2 s) ────────────────────────────────────────────
    {
        let state = Arc::clone(&state);
        thread::spawn(move || loop {
            let ram  = sysinfo::ram_percent();
            let vram = vram::read_pct();
            let mut s = state.lock().unwrap();
            s.ram_pct  = ram;
            s.vram_pct = vram;
            drop(s);
            thread::sleep(Duration::from_secs(2));
        });
    }

    // ── Headset battery (every 50 ms non-blocking) ────────────────────────
    {
        let state     = Arc::clone(&state);
        let screen_on = Arc::clone(&screen_on);
        thread::spawn(move || {
            let mut dev: Option<hidapi::HidDevice> = None;
            loop {
                if !screen_on.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(500));
                    continue;
                }
                if dev.is_none() {
                    if let Ok(api) = HidApi::new() {
                        for info in api.device_list() {
                            if info.vendor_id() == VID
                                && info.product_id() == PID
                                && info.usage_page() == BATT_USAGE_PAGE
                            {
                                if let Ok(d) = info.open_device(&api) {
                                    let _ = d.set_blocking_mode(false);
                                    dev = Some(d);
                                    break;
                                }
                            }
                        }
                    }
                    if dev.is_none() {
                        thread::sleep(Duration::from_secs(5));
                        continue;
                    }
                }
                let mut buf = [0u8; 64];
                match dev.as_ref().unwrap().read(&mut buf) {
                    Ok(n) if n >= 3 && buf[1] == 0xB7 => {
                        let p = buf[2];
                        if p <= 100 {
                            state.lock().unwrap().headset_pct = Some(p);
                        }
                    }
                    Err(_) => { dev = None; }
                    _ => {}
                }
                thread::sleep(Duration::from_secs(60));
            }
        });
    }

    // ── G703 battery (every 60 s) ─────────────────────────────────────────
    {
        let state     = Arc::clone(&state);
        let screen_on = Arc::clone(&screen_on);
        thread::spawn(move || loop {
            if screen_on.load(Ordering::Relaxed) {
                if let Some(pct) = g703::read() {
                    state.lock().unwrap().mouse_pct = Some(pct);
                }
            }
            thread::sleep(Duration::from_secs(60));
        });
    }

    // ── Monitor sleep watcher ─────────────────────────────────────────────
    {
        let screen_on = Arc::clone(&screen_on);
        thread::spawn(move || monitor::run(screen_on));
    }

    // ── ntfy.sh shutdown listener ─────────────────────────────────────────
    thread::spawn(|| {
        loop {
            if let Ok(resp) = ureq::get("https://ntfy.sh/shutdown-almito-king420/raw").call() {
                use std::io::BufRead;
                let reader = std::io::BufReader::new(resp.into_reader());
                for line in reader.lines() {
                    if let Ok(l) = line {
                        if !l.trim().is_empty() {
                            let _ = std::process::Command::new("shutdown")
                                .args(["/s", "/t", "0"])
                                .spawn();
                            return;
                        }
                    }
                }
            }
            thread::sleep(Duration::from_secs(5));
        }
    });

    // ── Main render loop ──────────────────────────────────────────────────
    let cfg      = config::load();
    let renderer = Renderer::new();
    let black    = Bitmap::new(128, 64, false);
    let mut dev  = connect_with_retry();

    if let Some(ref d) = dev {
        let _ = d.set_brightness(config::brightness_to_hw(cfg.brightness));
    }

    let mut shift_idx  = 0usize;
    let mut shift_tick = 0usize;

    loop {
        if !screen_on.load(Ordering::Relaxed) {
            if let Some(ref d) = dev { let _ = d.draw(&black, 0, 0); }
            thread::sleep(Duration::from_secs(1));
            continue;
        }
        if dev.is_none() {
            thread::sleep(Duration::from_secs(2));
            dev = connect_with_retry();
            if let Some(ref d) = dev {
                let _ = d.set_brightness(config::brightness_to_hw(cfg.brightness));
            }
            continue;
        }

        let s = *state.lock().unwrap();
        let frame = renderer.render(
            s.headset_pct, s.mouse_pct, s.ram_pct, s.vram_pct,
            s.argus.cpu_temp, s.argus.cpu_load,
            s.argus.gpu_temp, s.argus.gpu_load,
        );

        shift_tick += 1;
        if shift_tick >= SHIFT_EVERY {
            shift_tick = 0;
            shift_idx = (shift_idx + 1) % SHIFTS.len();
        }
        let (sx, sy) = SHIFTS[shift_idx];

        if dev.as_ref().unwrap().draw(&frame, sx, sy).is_err() {
            dev = None;
        }

        spin_sleep::sleep(Duration::from_millis(33));
    }
}

fn connect_with_retry() -> Option<Device> {
    for _ in 0..3 {
        if let Ok(d) = Device::connect() { return Some(d); }
        thread::sleep(Duration::from_secs(1));
    }
    None
}
