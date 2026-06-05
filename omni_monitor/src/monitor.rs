use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use winapi::shared::guiddef::GUID;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::{
    RegisterPowerSettingNotification,
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG,
    RegisterClassExW, TranslateMessage, WNDCLASSEXW, HWND_MESSAGE,
    WM_POWERBROADCAST,
};

// GUID_MONITOR_POWER_ON = {6FE69556-704A-47A0-8F24-C28D936FDA47}
const GUID_MONITOR_POWER_ON: GUID = GUID {
    Data1: 0x6FE6_9556,
    Data2: 0x704A,
    Data3: 0x47A0,
    Data4: [0x8F, 0x24, 0xC2, 0x8D, 0x93, 0x6F, 0xDA, 0x47],
};

const PBT_POWERSETTINGCHANGE: WPARAM = 0x8013;
const DEVICE_NOTIFY_WINDOW_HANDLE: winapi::shared::minwindef::DWORD = 0;

#[repr(C)]
struct PowerbroadcastSetting {
    _guid:        GUID,
    data_length: u32,
    data:        u32,
}

static mut SCREEN_ON_PTR: *const AtomicBool = std::ptr::null();

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_POWERBROADCAST && wp == PBT_POWERSETTINGCHANGE {
        unsafe {
            let ps = &*(lp as *const PowerbroadcastSetting);
            if !SCREEN_ON_PTR.is_null() {
                (*SCREEN_ON_PTR).store(ps.data != 0, Ordering::Relaxed);
            }
        }
    }
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

pub fn run(screen_on: Arc<AtomicBool>) {
    unsafe {
        SCREEN_ON_PTR = Arc::as_ptr(&screen_on);

        let class: Vec<u16> = "OmniMonWatch\0".encode_utf16().collect();
        let h_inst = GetModuleHandleW(std::ptr::null());

        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize      = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(wnd_proc);
        wc.hInstance   = h_inst;
        wc.lpszClassName = class.as_ptr();
        RegisterClassExW(&wc);

        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            std::ptr::null(),
            0,
            0, 0, 0, 0,
            HWND_MESSAGE,
            std::ptr::null_mut(),
            h_inst,
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            return;
        }

        RegisterPowerSettingNotification(
            hwnd as *mut _,
            &GUID_MONITOR_POWER_ON,
            DEVICE_NOTIFY_WINDOW_HANDLE,
        );

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
