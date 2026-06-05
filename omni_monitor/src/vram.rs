// GPU VRAM usage percentage:
//   Used  ← Windows PDH: \GPU Adapter Memory(*)\Dedicated Usage  (bytes)
//   Total ← DXGI IDXGIAdapter::GetDesc → DedicatedVideoMemory    (bytes)

use std::ptr;
use std::sync::OnceLock;
use winapi::shared::dxgi::{IDXGIAdapter, IDXGIFactory1, IID_IDXGIFactory1, DXGI_ADAPTER_DESC};
use winapi::um::libloaderapi::{GetProcAddress, LoadLibraryW};
use winapi::um::pdh::*;

const PDH_FMT_LARGE: u32 = 0x00000400;
const PDH_MORE_DATA: i32 = 0x800007D2u32 as i32;

// Cache total VRAM (doesn't change at runtime)
static TOTAL_VRAM_MB: OnceLock<u64> = OnceLock::new();

fn total_vram_mb() -> u64 {
    *TOTAL_VRAM_MB.get_or_init(|| unsafe { query_total_vram_mb().unwrap_or(0) })
}

type CreateDXGIFactory1Fn = unsafe extern "system" fn(
    *const winapi::shared::guiddef::GUID,
    *mut *mut winapi::ctypes::c_void,
) -> i32;

unsafe fn query_total_vram_mb() -> Option<u64> {
    let dll: Vec<u16> = "dxgi.dll\0".encode_utf16().collect();
    let lib = unsafe { LoadLibraryW(dll.as_ptr()) };
    if lib.is_null() { return None; }

    let sym = unsafe { GetProcAddress(lib, b"CreateDXGIFactory1\0".as_ptr() as *const i8) };
    if sym.is_null() { return None; }

    let create: CreateDXGIFactory1Fn = std::mem::transmute(sym);

    let mut factory: *mut IDXGIFactory1 = ptr::null_mut();
    if unsafe { create(&IID_IDXGIFactory1, &mut factory as *mut _ as *mut _) } != 0 {
        return None;
    }

    // Find the adapter with the most dedicated VRAM (the discrete GPU)
    let mut best_mb: u64 = 0;
    let mut idx = 0u32;
    loop {
        let mut adapter: *mut IDXGIAdapter = ptr::null_mut();
        if unsafe { (*factory).EnumAdapters(idx, &mut adapter) } != 0 { break; }
        let mut desc: DXGI_ADAPTER_DESC = std::mem::zeroed();
        if unsafe { (*adapter).GetDesc(&mut desc) } == 0 {
            let mb = desc.DedicatedVideoMemory as u64 / (1024 * 1024);
            if mb > best_mb { best_mb = mb; }
        }
        unsafe { (*adapter).Release() };
        idx += 1;
    }
    unsafe { (*factory).Release() };

    if best_mb > 0 { Some(best_mb) } else { None }
}

pub fn read_pct() -> Option<u32> {
    let total = total_vram_mb();
    if total == 0 { return None; }
    let used = read_used_mb()?;
    Some(((used * 100) / total).min(100) as u32)
}

fn read_used_mb() -> Option<u64> {
    unsafe { read_used_inner() }
}

unsafe fn read_used_inner() -> Option<u64> {
    let mut query: PDH_HQUERY = ptr::null_mut();
    if unsafe { PdhOpenQueryW(ptr::null(), 0, &mut query) } != 0 { return None; }

    let path: Vec<u16> = "\\GPU Adapter Memory(*)\\Dedicated Usage\0"
        .encode_utf16().collect();
    let mut counter: PDH_HCOUNTER = ptr::null_mut();
    if unsafe { PdhAddCounterW(query, path.as_ptr(), 0, &mut counter) } != 0 {
        unsafe { PdhCloseQuery(query) };
        return None;
    }

    unsafe { PdhCollectQueryData(query) };

    let mut buf_size: u32 = 0;
    let mut item_count: u32 = 0;
    let r = unsafe {
        PdhGetFormattedCounterArrayW(counter, PDH_FMT_LARGE,
            &mut buf_size, &mut item_count, ptr::null_mut())
    };
    if r != PDH_MORE_DATA || buf_size == 0 {
        unsafe { PdhCloseQuery(query) };
        return None;
    }

    let mut buf: Vec<u8> = vec![0u8; buf_size as usize];
    let r = unsafe {
        PdhGetFormattedCounterArrayW(counter, PDH_FMT_LARGE,
            &mut buf_size, &mut item_count,
            buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W)
    };
    unsafe { PdhCloseQuery(query) };
    if r != 0 || item_count == 0 { return None; }

    let items = std::slice::from_raw_parts(
        buf.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
        item_count as usize,
    );
    let mut total: i64 = 0;
    for item in items {
        if item.FmtValue.CStatus == 0 {
            total += unsafe { *item.FmtValue.u.largeValue() };
        }
    }
    if total > 0 { Some(total as u64 / (1024 * 1024)) } else { None }
}
