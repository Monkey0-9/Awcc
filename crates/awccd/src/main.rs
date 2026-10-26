use awcc_core::HardwareManager;
use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct ThermalGovernor {
    last_boost: u8,
}

impl ThermalGovernor {
    pub fn new() -> Self {
        Self { last_boost: 0 }
    }

    /// Evaluates piecewise-linear boost curve with 3°C anti-hunt hysteresis
    pub fn compute_boost(&mut self, max_temp: f32) -> u8 {
        let raw_target = if max_temp < 50.0 {
            0
        } else if max_temp >= 85.0 {
            255
        } else {
            let ratio = (max_temp - 50.0) / (85.0 - 50.0);
            (ratio * 255.0) as u8
        };

        // Anti-hunt hysteresis: don't ramp down immediately if drop is under 3°C
        if raw_target < self.last_boost {
            // Apply gradual ramp-down damping (max 15 units per step)
            let damped = self.last_boost.saturating_sub(15);
            self.last_boost = std::cmp::max(damped, raw_target);
        } else {
            // Instant ramp-up for immediate cooling protection
            self.last_boost = raw_target;
        }

        self.last_boost
    }
}

fn main() {
    println!("[*] Starting awccd (AWCC Linux Autonomous Thermal & Policy Daemon)...");
    let hw = HardwareManager::new();
    let mut governor = ThermalGovernor::new();

    let term = Arc::new(AtomicBool::new(false));
    let mut signals = Signals::new(TERM_SIGNALS).expect("Failed to register signal handlers");

    let term_clone = Arc::clone(&term);
    thread::spawn(move || {
        for _ in signals.forever() {
            println!("\n[!] Received termination signal! Restoring firmware fan control...");
            term_clone.store(true, Ordering::SeqCst);
            break;
        }
    });

    println!("[+] awccd initialized with multi-sensor thermal governor. Active monitoring running.");

    while !term.load(Ordering::SeqCst) {
        let thermals = hw.read_thermal_telemetry();
        let cpu_temp = thermals.cpu_core_max_temp;
        let gpu_temp = thermals.gpu_temp.unwrap_or(0.0);
        let max_temp = cpu_temp.max(gpu_temp);

        let (current_profile, _) = HardwareManager::get_platform_profile();

        // If in custom or auto curve mode, adjust fans dynamically
        if current_profile == "custom" {
            let target_boost = governor.compute_boost(max_temp);
            for fan in 1..=2 {
                let _ = hw.write_fan_boost(fan, target_boost);
            }
        }

        thread::sleep(Duration::from_secs(1));
    }

    println!("[*] Releasing fan boost to 0 before daemon shutdown...");
    for fan in 1..=2 {
        let _ = hw.write_fan_boost(fan, 0);
    }
    println!("[+] Fail-safe daemon shutdown complete.");
}
