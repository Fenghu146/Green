//! What the machine was doing when a benchmark took its number (design §5.4).

/// The machine's busy share since it booted, from 0 to 1; `None` where it is not
/// read. Windows counts kernel time, idle time included, against user time.
#[cfg(windows)]
pub(crate) fn busy() -> Option<f64> {
    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
    }
    let ticks = |time: FileTime| (u64::from(time.high) << 32) | u64::from(time.low);
    let zero = || FileTime { low: 0, high: 0 };
    let (mut idle, mut kernel, mut user) = (zero(), zero(), zero());
    // SAFETY: three FILETIME outs in the order kernel32 documents.
    let read = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    if read == 0 {
        return None;
    }
    let total = ticks(kernel) + ticks(user);
    (total > 0).then(|| 1. - ticks(idle) as f64 / total as f64)
}

#[cfg(not(windows))]
pub(crate) fn busy() -> Option<f64> {
    None
}

/// The line a benchmark prints before its number: its cores, and how busy the
/// machine was. A number taken while other builds ran is not a regression.
pub(crate) fn machine() -> String {
    let cores = std::thread::available_parallelism().map_or(0, |n| n.get());
    match busy() {
        Some(busy) => format!("{cores} logical cores, machine busy {:.0}%", busy * 100.),
        None => format!("{cores} logical cores"),
    }
}
