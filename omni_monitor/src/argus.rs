use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use winapi::shared::minwindef::FALSE;
use winapi::um::handleapi::CloseHandle;
use winapi::um::memoryapi::{FILE_MAP_READ, MapViewOfFile, OpenFileMappingW, UnmapViewOfFile};

const ARGUS_SM: &str = "Global\\ARGUSMONITOR_DATA_INTERFACE";
const ARGUS_SIG: u32 = 0x4D677241;
const HDR_OFFSET: usize = 232;
const SENSOR_SZ: usize = 212;
const OFF_OFFSET: usize = 20;
const CNT_OFFSET: usize = 128;
const VAL_OFFSET: usize = 196;

const CPU_TEMP: usize = 6;
const GPU_TEMP: usize = 10;
const GPU_LOAD: usize = 12;
const CPU_LOAD: usize = 23;

#[derive(Debug, Default, Clone, Copy)]
pub struct ArgusData {
    pub cpu_temp: Option<f64>,
    pub cpu_load: Option<f64>,
    pub gpu_temp: Option<f64>,
    pub gpu_load: Option<f64>,
}

pub fn read() -> ArgusData {
    unsafe { read_inner().unwrap_or_default() }
}

unsafe fn read_inner() -> Option<ArgusData> {
    let wide: Vec<u16> = OsStr::new(ARGUS_SM).encode_wide().chain(Some(0)).collect();
    let h = unsafe { OpenFileMappingW(FILE_MAP_READ, FALSE, wide.as_ptr()) };
    if h.is_null() { return None; }
    let view = unsafe { MapViewOfFile(h, FILE_MAP_READ, 0, 0, 0) };
    if view.is_null() { unsafe { CloseHandle(h) }; return None; }

    let buf_size = HDR_OFFSET + SENSOR_SZ * 512;
    let raw = unsafe { std::slice::from_raw_parts(view as *const u8, buf_size) };
    let result = parse(raw);

    unsafe { UnmapViewOfFile(view) };
    unsafe { CloseHandle(h) };
    result
}

fn parse(raw: &[u8]) -> Option<ArgusData> {
    if raw.len() < 4 { return None; }
    if u32::from_le_bytes(raw[0..4].try_into().ok()?) != ARGUS_SIG { return None; }

    let mut data = ArgusData::default();

    // field: 0=cpu_temp 1=gpu_temp 2=gpu_load 3=cpu_load 4=ram_usage
    // is_temp: skip 0.0 (invalid for temperatures, valid for loads/usage)
    for &(type_id, field, is_temp) in &[
        (CPU_TEMP, 0u8, true),
        (GPU_TEMP, 1,   true),
        (GPU_LOAD, 2,   false),
        (CPU_LOAD, 3,   false),
    ] {
        let off_base = OFF_OFFSET + type_id * 4;
        let cnt_base = CNT_OFFSET + type_id * 4;
        if off_base + 4 > raw.len() || cnt_base + 4 > raw.len() { continue; }

        let offset = u32::from_le_bytes(raw[off_base..off_base + 4].try_into().ok()?) as usize;
        let count  = u32::from_le_bytes(raw[cnt_base..cnt_base + 4].try_into().ok()?) as usize;
        if count == 0 { continue; }

        let vp = HDR_OFFSET + offset * SENSOR_SZ + VAL_OFFSET;
        if vp + 8 > raw.len() { continue; }

        let val = f64::from_le_bytes(raw[vp..vp + 8].try_into().ok()?);
        if val.is_nan() || (is_temp && val == 0.0) { continue; }

        match field {
            0 => data.cpu_temp  = Some(val),
            1 => data.gpu_temp  = Some(val),
            2 => data.gpu_load  = Some(val),
            3 => data.cpu_load  = Some(val),
            _ => {}
        }
    }

    Some(data)
}
