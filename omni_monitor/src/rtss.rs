// RivaTuner Statistics Server shared memory FPS reader.
// Shared memory: "Global\\RTSSSharedMemoryV2"
//
// Header:
//   [0]  u32 signature (0x52545353 = "RTSS")
//   [4]  u32 version
//   [8]  u32 app_entry_size
//   [12] u32 app_arr_offset
//   [16] u32 app_arr_size
//
// Per-app entry:
//   [0]   u32   process_id
//   [4]   [u8;260] name
//   [264] u32   flags
//   [268] u32   time0 (ms)
//   [272] u32   time1 (ms)
//   [276] u32   frames
//   [280] u32   osd_x
//   [284] u32   osd_y
//   [288] u32   osd_pixel
//   [292] u32   osd_color
//   [296] u32   osd_frame
//   [300] f32   fFPS   ← direct FPS value, always up to date

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use winapi::shared::minwindef::FALSE;
use winapi::um::handleapi::CloseHandle;
use winapi::um::memoryapi::{FILE_MAP_READ, MapViewOfFile, OpenFileMappingW, UnmapViewOfFile};

// RTSS uses session-local shared memory (Local\) not Global\
const RTSS_NAMES: &[&str] = &[
    "Local\\RTSSSharedMemoryV2",
    "Global\\RTSSSharedMemoryV2",
];
const RTSS_SIG: u32 = 0x52545353;
const OFFSET_FPS: usize = 300;

pub fn read() -> Option<u32> {
    unsafe { read_inner() }
}

unsafe fn read_inner() -> Option<u32> {
    let h = RTSS_NAMES.iter().find_map(|name| {
        let wide: Vec<u16> = OsStr::new(name).encode_wide().chain(Some(0)).collect();
        let h = unsafe { OpenFileMappingW(FILE_MAP_READ, FALSE, wide.as_ptr()) };
        if h.is_null() { None } else { Some(h) }
    })?;

    let view = unsafe { MapViewOfFile(h, FILE_MAP_READ, 0, 0, 0) };
    if view.is_null() { unsafe { CloseHandle(h) }; return None; }

    let result = parse(view as *const u8);
    unsafe { UnmapViewOfFile(view) };
    unsafe { CloseHandle(h) };
    result
}

fn ru32(base: *const u8, off: usize) -> u32 {
    unsafe { u32::from_le_bytes(*(base.add(off) as *const [u8; 4])) }
}

fn rf32(base: *const u8, off: usize) -> f32 {
    unsafe { f32::from_le_bytes(*(base.add(off) as *const [u8; 4])) }
}

unsafe fn parse(base: *const u8) -> Option<u32> {
    if ru32(base, 0) != RTSS_SIG { return None; }

    let entry_size = ru32(base, 8)  as usize;
    let arr_off    = ru32(base, 12) as usize;
    let arr_size   = ru32(base, 16) as usize;

    if entry_size == 0 || arr_size == 0 { return None; }

    let mut best: Option<u32> = None;

    for i in 0..arr_size {
        let entry = base.add(arr_off + i * entry_size);
        let pid   = ru32(entry, 0);
        if pid == 0 { continue; }

        // Try direct fFPS field first (most reliable)
        if entry_size > OFFSET_FPS + 4 {
            let fps_f = rf32(entry, OFFSET_FPS);
            if fps_f.is_finite() && fps_f >= 1.0 && fps_f < 1000.0 {
                let fps = fps_f.round() as u32;
                best = Some(best.map_or(fps, |b: u32| b.max(fps)));
                continue;
            }
        }

        // Fallback: derive from frame counter / elapsed time
        let time0  = ru32(entry, 268);
        let time1  = ru32(entry, 272);
        let frames = ru32(entry, 276);
        if time1 > time0 && frames > 0 {
            let dt = time1.wrapping_sub(time0) as f64;
            let fps = (frames as f64 * 1000.0 / dt).round() as u32;
            if fps >= 1 && fps < 1000 {
                best = Some(best.map_or(fps, |b: u32| b.max(fps)));
            }
        }
    }

    best
}
