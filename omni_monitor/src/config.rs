use std::path::PathBuf;

pub struct Config {
    /// Display brightness 0–100 %
    pub brightness: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self { brightness: 100 }
    }
}

/// Returns the directory containing the running exe.
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn load() -> Config {
    let path = exe_dir().join("omni_monitor.conf");

    // Create default config if missing
    if !path.exists() {
        let _ = std::fs::write(
            &path,
            "# Omni Monitor config\n\
             # brightness: 0-100 (percent)\n\
             brightness=100\n",
        );
        return Config::default();
    }

    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Config::default(),
    };

    let mut cfg = Config::default();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() { continue; }
        if let Some((key, val)) = line.split_once('=') {
            if key.trim() == "brightness" {
                if let Ok(n) = val.trim().parse::<u8>() {
                    cfg.brightness = n.min(100);
                }
            }
        }
    }
    cfg
}

/// Convert 0–100 % → 1–10 (ggoled_lib brightness scale)
pub fn brightness_to_hw(pct: u8) -> u8 {
    let v = ((pct as f32 / 100.0) * 9.0).round() as u8 + 1;
    v.min(10).max(1)
}
