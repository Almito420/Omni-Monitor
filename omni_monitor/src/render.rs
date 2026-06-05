// PixelOperator.ttf at 16px — native bitmap size, pixel-perfect on OLED.
//
// Layout (ggoled 128×64, high y = physical top):
//   Physical TOP    y≈46  Battery row:  Omni% | G703%
//   Physical MID    y≈4   CPU row:      load% | temp° | RAM%
//   Physical BOT    y≈25  GPU row:      load% | temp° | FPS

use fontdue::{Font, FontSettings};
use ggoled_lib::Bitmap;

const FONT_SIZE: f32 = 16.0;

// Columns
const X_LABEL: usize = 2;    // "CPU" / "GPU" / "Omni"
const X_VALUE: usize = 34;   // load %
const X_TEMP:  usize = 68;   // temperature °
const X_RIGHT: usize = 98;   // RAM % (CPU row) / FPS (GPU row)
const X_G703:  usize = 68;   // "G703" label
const X_G703V: usize = 99;   // G703 battery %

// Rows
const Y_CPU:  usize = 4;
const Y_GPU:  usize = 25;
const Y_BATT: usize = 46;

pub struct Renderer {
    font: Font,
}

impl Renderer {
    pub fn new() -> Self {
        let settings = FontSettings { scale: FONT_SIZE, ..FontSettings::default() };
        let font = Font::from_bytes(
            include_bytes!("../../ggoled_draw/fonts/PixelOperator.ttf").to_vec(),
            settings,
        ).unwrap();
        Self { font }
    }

    pub fn render(
        &self,
        h_pct:    Option<u8>,
        m_pct:    Option<u8>,
        ram_pct:  Option<u32>,
        vram_pct: Option<u32>,
        cpu_temp: Option<f64>,
        cpu_load: Option<f64>,
        gpu_temp: Option<f64>,
        gpu_load: Option<f64>,
    ) -> Bitmap {
        let mut bmp = Bitmap::new(128, 64, false);

        // CPU row: load | temp | RAM%
        self.put(&mut bmp, X_LABEL, Y_CPU, "CPU");
        self.put(&mut bmp, X_VALUE, Y_CPU, &fmt_pct(cpu_load));
        self.put(&mut bmp, X_TEMP,  Y_CPU, &fmt_temp(cpu_temp));
        if let Some(r) = ram_pct {
            self.put(&mut bmp, X_RIGHT, Y_CPU, &format!("{}%", r));
        }

        // GPU row: load | temp | VRAM%
        self.put(&mut bmp, X_LABEL, Y_GPU, "GPU");
        self.put(&mut bmp, X_VALUE, Y_GPU, &fmt_pct(gpu_load));
        self.put(&mut bmp, X_TEMP,  Y_GPU, &fmt_temp(gpu_temp));
        if let Some(v) = vram_pct {
            self.put(&mut bmp, X_RIGHT, Y_GPU, &format!("{}%", v));
        }

        // Battery row
        self.put(&mut bmp, X_LABEL, Y_BATT, "Omni");
        self.put(&mut bmp, X_VALUE, Y_BATT, &fmt_batt(h_pct));
        self.put(&mut bmp, X_G703,  Y_BATT, "G703");
        self.put(&mut bmp, X_G703V, Y_BATT, &fmt_batt(m_pct));

        bmp
    }

    fn put(&self, bmp: &mut Bitmap, x: usize, y: usize, text: &str) {
        let lm = match self.font.horizontal_line_metrics(FONT_SIZE) {
            Some(m) => m,
            None => return,
        };
        let baseline = y as f32 + lm.ascent;
        let mut pen_x = x as f32;

        for ch in text.chars() {
            let (metrics, pixels) = self.font.rasterize(ch, FONT_SIZE);
            if metrics.width == 0 || metrics.height == 0 {
                pen_x += metrics.advance_width;
                continue;
            }
            let gx0 = (pen_x + metrics.xmin as f32).round() as isize;
            let gy0 = (baseline - metrics.ymin as f32 - metrics.height as f32).round() as isize;

            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    if pixels[row * metrics.width + col] >= 64 {
                        let px = gx0 + col as isize;
                        let py = gy0 + row as isize;
                        if px >= 0 && py >= 0 {
                            let px = px as usize;
                            let py = py as usize;
                            if px < bmp.w && py < bmp.h {
                                bmp.data.set(py * bmp.w + px, true);
                            }
                        }
                    }
                }
            }
            pen_x += metrics.advance_width;
        }
    }
}

fn fmt_pct(v: Option<f64>) -> String {
    match v {
        None => "--".into(),
        Some(x) => {
            // Prevent -0% from floating point rounding artifacts
            let n = x.max(0.0).round() as u32;
            format!("{}%", n)
        }
    }
}
fn fmt_temp(v: Option<f64>) -> String {
    v.map_or("--".into(), |x| format!("{:.0}\u{00B0}", x))
}
fn fmt_batt(v: Option<u8>) -> String {
    v.map_or("--".into(), |p| format!("{}%", p))
}
