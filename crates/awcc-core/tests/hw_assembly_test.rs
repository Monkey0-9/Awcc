use awcc_core::hw::{
    cycles_to_nanos, get_or_calibrate_tsc_freq, measure_cycles, probe_cpu_features, read_tsc,
};

#[test]
fn test_cpu_probe_features() {
    let features = probe_cpu_features();
    #[cfg(target_arch = "x86_64")]
    {
        // On any modern x86_64 host, vendor should be detected (Intel or AMD)
        assert!(
            !features.vendor_id.is_empty(),
            "CPUID vendor string must be non-empty"
        );
        // TSC/RDTSCP should be universally supported on modern CPUs
        assert!(
            features.rdtscp_supported,
            "RDTSCP must be supported on modern x86_64"
        );
    }
    let _ = features;
}

#[test]
fn test_rdtsc_cycle_measurement() {
    let tsc1 = read_tsc();
    let (val, cycles) = measure_cycles(|| {
        let mut sum = 0u64;
        for i in 0..1000 {
            sum = sum.wrapping_add(i);
        }
        sum
    });
    let tsc2 = read_tsc();

    assert_eq!(val, 499500);
    assert!(cycles > 0, "Measured cycle delta must be greater than zero");
    assert!(tsc2 >= tsc1, "TSC must be monotonically non-decreasing");

    let freq = get_or_calibrate_tsc_freq();
    assert!(
        freq > 500_000_000,
        "TSC frequency should be reasonable (>500 MHz)"
    );

    let nanos = cycles_to_nanos(cycles);
    assert!(
        nanos < 10_000_000,
        "1000 iterations must take less than 10 milliseconds"
    );
}
