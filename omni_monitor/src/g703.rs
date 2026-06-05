// G703 LIGHTSPEED battery via HID++ 2.0 feature 0x1001 (Battery Voltage).
// Ported from LGSTrayBattery / Omni.py.

use hidapi::HidApi;
use std::time::{Duration, Instant};

const LOGI_VID: u16 = 0x046d;
const LOGI_SW_ID: u8 = 0x0A;

// Battery1001.cs LUT: voltage (mV) → percentage.
// Index 0 = 100%, index 99 = 1%.  Count entries strictly less than mv.
const VOLT_LUT: [u16; 100] = [
    4186,4156,4143,4133,4122,4113,4103,4094,4086,4075,
    4067,4059,4051,4043,4035,4027,4019,4011,4003,3997,
    3989,3983,3976,3969,3961,3955,3949,3942,3935,3929,
    3922,3916,3909,3902,3896,3890,3883,3877,3870,3865,
    3859,3853,3848,3842,3837,3833,3828,3824,3819,3815,
    3811,3808,3804,3800,3797,3793,3790,3787,3784,3781,
    3778,3775,3772,3770,3767,3764,3762,3759,3757,3754,
    3751,3748,3744,3741,3737,3734,3730,3726,3724,3720,
    3717,3714,3710,3706,3702,3697,3693,3688,3683,3677,
    3671,3666,3662,3658,3654,3646,3633,3612,3579,3537,
];

fn mv_to_pct(mv: u16) -> u8 {
    VOLT_LUT.iter().filter(|&&v| v < mv).count() as u8
}

fn norm(r: &[u8]) -> &[u8] {
    if !r.is_empty() && r[0] != 0x10 && r[0] != 0x11
        && r.len() >= 2 && (r[1] == 0x10 || r[1] == 0x11)
    {
        &r[1..]
    } else {
        r
    }
}

// Write to SHORT, read from SHORT + LONG until we get a matching response.
fn xchg(
    sdev: &hidapi::HidDevice,
    ldev: Option<&hidapi::HidDevice>,
    msg: &[u8],
    timeout: Duration,
) -> Option<Vec<u8>> {
    let feat_idx = msg[2];
    let mut buf = [0u8; 20];

    // Flush
    let _ = sdev.set_blocking_mode(false);
    while sdev.read(&mut buf).ok().map_or(false, |n| n > 0) {}
    if let Some(l) = ldev {
        let _ = l.set_blocking_mode(false);
        while l.read(&mut buf).ok().map_or(false, |n| n > 0) {}
    }

    let _ = sdev.write(msg);

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        for dev in [Some(sdev), ldev].into_iter().flatten() {
            if let Ok(n) = dev.read(&mut buf) {
                if n == 0 { continue; }
                let r = norm(&buf[..n]);
                if r.len() < 6 { continue; }
                if r[2] == 0x8F { continue; } // error
                if r[2] == feat_idx && (r[3] & 0x0F) == LOGI_SW_ID {
                    return Some(r.to_vec());
                }
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

pub fn read() -> Option<u8> {
    let api = HidApi::new().ok()?;
    let mut sp: Option<std::ffi::CString> = None;
    let mut lp: Option<std::ffi::CString> = None;

    for info in api.device_list() {
        if info.vendor_id() != LOGI_VID { continue; }
        let up = info.usage_page();
        let u  = info.usage();
        if (up & 0xFF00) == 0xFF00 {
            if u == 0x0001 && sp.is_none() {
                sp = Some(info.path().to_owned());
            } else if u == 0x0002 && lp.is_none() {
                lp = Some(info.path().to_owned());
            }
        }
    }

    let sdev = api.open_path(sp?.as_c_str()).ok()?;
    let _ = sdev.set_blocking_mode(false);

    let ldev = lp.and_then(|p| api.open_path(p.as_c_str()).ok());

    // Force device arrival, read dev_idx
    let _ = sdev.write(&[0x10, 0xFF, 0x80, 0x02, 0x02, 0x00, 0x00]);
    std::thread::sleep(Duration::from_millis(500));

    let mut dev_idx: u8 = 1;
    let mut buf = [0u8; 20];
    for _ in 0..20 {
        if let Ok(n) = sdev.read(&mut buf) {
            if n >= 3 && buf[0] == 0x10 && buf[2] == 0x41 && buf[1] > 0 && buf[1] < 0xFF {
                dev_idx = buf[1];
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let t = Duration::from_millis(1500);

    // Get IFeatureSet index (feature 0x0001)
    let r = xchg(&sdev, ldev.as_ref(), &[0x10, dev_idx, 0x00, LOGI_SW_ID, 0x00, 0x01, 0x00], t)?;
    if r[4] == 0 { return None; }
    let fi_fs = r[4];

    // Get feature count
    let r = xchg(&sdev, ldev.as_ref(), &[0x10, dev_idx, fi_fs, LOGI_SW_ID, 0x00, 0x00, 0x00], t)?;
    let count = r[4];

    // Find feature 0x1001 (Battery Voltage)
    let mut fi_batt: u8 = 0;
    for i in 1..=count {
        let msg = [0x10, dev_idx, fi_fs, (0x01 << 4) | LOGI_SW_ID, i, 0x00, 0x00];
        if let Some(r) = xchg(&sdev, ldev.as_ref(), &msg, Duration::from_millis(500)) {
            if r.len() >= 6 && ((r[4] as u16) << 8 | r[5] as u16) == 0x1001 {
                fi_batt = i;
                break;
            }
        }
    }
    if fi_batt == 0 { return None; }

    // Query voltage
    let r = xchg(&sdev, ldev.as_ref(), &[0x10, dev_idx, fi_batt, LOGI_SW_ID, 0x00, 0x00, 0x00], t)?;
    if r.len() < 6 { return None; }

    let mv  = (r[4] as u16) << 8 | r[5] as u16;
    Some(mv_to_pct(mv))
}
