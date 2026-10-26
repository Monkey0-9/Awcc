//! High-precision x86_64 RDTSC hardware cycle counter & microsecond timing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static CACHED_TSC_FREQ: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
pub fn read_tsc() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_mm_lfence();
        let tsc = core::arch::x86_64::_rdtsc();
        core::arch::x86_64::_mm_lfence();
        tsc
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        0
    }
}

#[inline(always)]
pub fn read_tscp(aux: &mut u32) -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let mut id = 0u32;
        let tsc = core::arch::x86_64::__rdtscp(&mut id as *mut u32);
        core::arch::x86_64::_mm_lfence();
        *aux = id;
        tsc
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        *aux = 0;
        0
    }
}

/// Measures the exact CPU cycle delta and execution duration of a closure.
#[inline(always)]
pub fn measure_cycles<F, R>(f: F) -> (R, u64)
where
    F: FnOnce() -> R,
{
    let start = read_tsc();
    let res = f();
    let end = read_tsc();
    let cycles = end.saturating_sub(start);
    (res, cycles)
}

/// Calibrates and returns the estimated TSC frequency in Hertz (cycles per second).
pub fn get_or_calibrate_tsc_freq() -> u64 {
    let cached = CACHED_TSC_FREQ.load(Ordering::Relaxed);
    if cached > 0 {
        return cached;
    }

    let sleep_dur = Duration::from_millis(10);
    let start_tsc = read_tsc();
    let start_instant = Instant::now();
    std::thread::sleep(sleep_dur);
    let end_tsc = read_tsc();
    let elapsed = start_instant.elapsed().as_secs_f64();

    let cycles = end_tsc.saturating_sub(start_tsc);
    let freq = if elapsed > 0.0 {
        (cycles as f64 / elapsed) as u64
    } else {
        2_500_000_000
    };

    let safe_freq = if freq > 100_000_000 {
        freq
    } else {
        2_500_000_000
    };
    CACHED_TSC_FREQ.store(safe_freq, Ordering::Relaxed);
    safe_freq
}

/// Converts CPU cycle count to nanoseconds using the calibrated TSC clock.
#[inline(always)]
pub fn cycles_to_nanos(cycles: u64) -> u64 {
    let freq = get_or_calibrate_tsc_freq();
    if freq == 0 {
        return 0;
    }
    ((cycles as u128 * 1_000_000_000) / freq as u128) as u64
}
