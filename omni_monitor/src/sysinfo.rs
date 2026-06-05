use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

/// Returns physical RAM usage as a percentage (0–100).
pub fn ram_percent() -> Option<u32> {
    unsafe {
        let mut ms: MEMORYSTATUSEX = std::mem::zeroed();
        ms.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut ms) != 0 {
            Some(ms.dwMemoryLoad)
        } else {
            None
        }
    }
}
