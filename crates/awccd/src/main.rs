use awcc_core::{HardwareManager, ThermalGovernor};
use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut sysfs_root: Option<PathBuf> = None;
    let mut run_once = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--sysfs-root" => {
                if i + 1 < args.len() {
                    sysfs_root = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                } else {
                    eprintln!("[-] Error: --sysfs-root requires a directory path");
                    std::process::exit(1);
                }
            }
            "--once" => {
                run_once = true;
            }
            "-h" | "--help" => {
                println!("AWCC Linux Thermal & Safety Governor Daemon (awccd)");
                println!("Usage: awccd [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --sysfs-root <PATH>  Specify alternative sysfs root (for testing)");
                println!("  --once               Run a single evaluation cycle and exit");
                println!("  -h, --help           Print help");
                return;
            }
            unknown => {
                eprintln!(
                    "[-] Error: Unknown option '{}'. Run with --help for usage.",
                    unknown
                );
                std::process::exit(1);
            }
        }
        i += 1;
    }

    println!("[*] Starting awccd (AWCC Linux Autonomous Thermal & Policy Daemon)...");
    let hw = match sysfs_root {
        Some(root) => HardwareManager::with_sysfs_root(root),
        None => HardwareManager::new(),
    };
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

    loop {
        if term.load(Ordering::SeqCst) {
            break;
        }

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
                        if let Err(e) = hw.set_fan_level(fan.fan_index, target_boost) {
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

        if run_once {
            break;
        }

        thread::sleep(Duration::from_secs(1));
    }

    println!("[*] Fail-safe shutdown: Releasing fans before exit...");
    if let Err(e) = hw.release_fans() {
        eprintln!("[-] Warning: Failed to release fans: {}", e);
    }
    println!("[+] Fail-safe daemon shutdown complete.");
}
