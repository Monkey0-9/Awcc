use awcc_core::HardwareManager;
use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct ThermalGovernor {
    last_boost: u8,
}

impl Default for ThermalGovernor {
    fn default() -> Self {
        Self::new()
    }
}

impl ThermalGovernor {
    pub fn new() -> Self {
        Self { last_boost: 0 }
    }

    /// Evaluates piecewise-linear boost curve with anti-hunt hysteresis
    pub fn compute_boost(&mut self, max_temp: f32) -> u8 {
        let raw_target = if max_temp < 50.0 {
            0
        } else if max_temp >= 85.0 {
            255
        } else {
            let ratio = (max_temp - 50.0) / (85.0 - 50.0);
            (ratio * 255.0) as u8
        };

        // Anti-hunt hysteresis: don't ramp down aggressively if drop is minor
        if raw_target < self.last_boost {
            // Apply gradual ramp-down damping (max 15 units per cycle)
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
        if signals.forever().next().is_some() {
            println!("\n[!] Received termination signal! Restoring firmware fan control...");
            term_clone.store(true, Ordering::SeqCst);
        }
    });

    println!(
        "[+] awccd initialized with multi-sensor thermal governor and failsafe safety policy."
    );

    while !term.load(Ordering::SeqCst) {
        let thermals = hw.read_thermal_telemetry();
        let cpu_temp = thermals.cpu_core_max_temp.or(thermals.cpu_pkg_temp);
        let gpu_temp = thermals.gpu_temp;

        let effective_temp = match (cpu_temp, gpu_temp) {
            (Some(c), Some(g)) => Some(c.max(g)),
            (Some(c), None) => Some(c),
            (None, Some(g)) => Some(g),
            (None, None) => None,
        };

        let (current_profile, _, _) = hw.get_platform_profile();
        let fans = hw.discover_fans();

        match effective_temp {
            Some(temp) => {
                // If in custom mode, run autonomous piecewise thermal curve
                if current_profile == "custom" {
                    let target_boost = governor.compute_boost(temp);
                    for fan in &fans {
                        if let Err(e) = hw.write_fan_boost(fan.fan_index, target_boost) {
                            eprintln!(
                                "[-] Error: Failed to actuate boost on fan {}: {}",
                                fan.fan_index, e
                            );
                        }
                    }
                }
            }
            None => {
                // Failsafe safety policy: Never assume 0°C if sensors are unavailable!
                // Release to firmware or log warning
                eprintln!("[!] Warning: Sensor telemetry unavailable; avoiding fan suppression.");
            }
        }

        thread::sleep(Duration::from_secs(1));
    }

    println!("[*] Fail-safe shutdown: Releasing fan boost to 0 before exit...");
    let fans = hw.discover_fans();
    for fan in &fans {
        if let Err(e) = hw.write_fan_boost(fan.fan_index, 0) {
            eprintln!(
                "[-] Warning: Failed to release fan {}: {}",
                fan.fan_index, e
            );
        }
    }
    println!("[+] Fail-safe daemon shutdown complete.");
}
