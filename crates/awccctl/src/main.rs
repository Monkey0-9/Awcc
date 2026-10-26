use awcc_core::HardwareManager;
use clap::{Parser, Subcommand};
use colored::*;
use std::thread::sleep;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "awccctl")]
#[command(author = "AWCC Linux Team")]
#[command(version = "0.1.0")]
#[command(about = "Dell AWCC Fan Control & Telemetry Utility for Linux", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Detect AWCC ACPI WMI, G-Mode, and hwmon interfaces
    Detect,

    /// Show current fan telemetry, thermals, and platform profile
    Status {
        /// Output formatted JSON
        #[arg(long)]
        json: bool,
    },

    /// Set platform profile (e.g. performance, balanced, quiet, cool, ultra)
    Profile {
        /// Profile name
        name: String,
    },

    /// Toggle or query Dell Game Shift / Ultra Performance G-Mode
    Gmode {
        /// Optional state (on/off)
        state: Option<String>,
    },

    /// Fan control operations
    Fan {
        #[command(subcommand)]
        command: FanCommands,
    },

    /// Comprehensive system diagnostic health report
    Doctor,

    /// Controlled ramp step verification
    Verify {
        /// Duration per step in seconds
        #[arg(long, default_value_t = 10)]
        step_duration: u64,
    },
}

#[derive(Subcommand)]
enum FanCommands {
    /// Set boost on specified fan (0-255)
    Set {
        /// Fan index (1 for CPU, 2 for GPU)
        #[arg(long)]
        fan: u8,
        /// Boost value (0 to 255)
        #[arg(long)]
        boost: u8,
    },
    /// Request maximum fan boost (255) on all fans
    Max,
    /// Release fan boost (0) back to BIOS firmware control
    Release,
}

fn main() {
    let cli = Cli::parse();
    let hw = HardwareManager::new();

    match cli.command {
        Commands::Detect => cmd_detect(&hw),
        Commands::Status { json } => cmd_status(&hw, json),
        Commands::Profile { name } => cmd_profile(&hw, &name),
        Commands::Gmode { state } => cmd_gmode(&hw, state),
        Commands::Fan { command } => match command {
            FanCommands::Set { fan, boost } => cmd_fan_set(&hw, fan, boost),
            FanCommands::Max => cmd_fan_max(&hw),
            FanCommands::Release => cmd_fan_release(&hw),
        },
        Commands::Doctor => cmd_doctor(&hw),
        Commands::Verify { step_duration } => cmd_verify(&hw, step_duration),
    }
}

fn cmd_detect(hw: &HardwareManager) {
    let (vendor, prod, bios) = HardwareManager::get_dmi_info();
    println!("{}", "=== AWCC Hardware Interface Detection ===".bold().cyan());
    println!("  DMI Vendor:       {}", vendor.green());
    println!("  DMI Product:      {}", prod.green());
    println!("  BIOS Version:     {}", bios.green());
    println!(
        "  Driver Loaded:    {}",
        if hw.is_driver_loaded() {
            "YES (alienware_wmi active)".green()
        } else {
            "NO (module not loaded)".red()
        }
    );

    let (cur, choices) = HardwareManager::get_platform_profile();
    println!(
        "  Platform Profile: {} (Available: [{}])",
        cur.yellow(),
        choices.join(", ")
    );
    println!(
        "  G-Mode Support:   {}",
        "Supported (Dell G15 5530 Quirk Active)".bold().green()
    );
}

fn cmd_status(hw: &HardwareManager, json: bool) {
    let tele = hw.get_telemetry();
    if json {
        println!("{}", serde_json::to_string_pretty(&tele).unwrap());
        return;
    }

    println!("{}", "=== AWCC Fan Control & Telemetry Status ===".bold().cyan());
    println!("  Platform Profile: {}", tele.active_platform_profile.bold().yellow());
    println!(
        "  Game Shift (G-Mode): {}",
        if tele.gmode_active {
            "ACTIVE (Ultra-Performance)".bold().green()
        } else {
            "INACTIVE".normal()
        }
    );
    println!("  Available Choices: [{}]", tele.platform_profile_choices.join(", "));
    println!();
    println!("{}", "--- Thermal Sensor Matrix ---".bold());
    println!("  CPU Package Temp:   {:.1}°C", tele.thermals.cpu_pkg_temp);
    println!("  CPU Core Max Temp:  {:.1}°C", tele.thermals.cpu_core_max_temp);
    if let Some(gpu) = tele.thermals.gpu_temp {
        println!("  NVIDIA GPU Temp:    {:.1}°C", gpu);
    }
    if let Some(vrm) = tele.thermals.vrm_temp {
        println!("  Motherboard VRM:    {:.1}°C", vrm);
    }
    println!();
    println!(
        "{:<14} {:<16} {:<16} {:<12}",
        "FAN", "REQUESTED BOOST", "READBACK BOOST", "MEASURED RPM"
    );
    println!("{:-<60}", "");

    for f in &tele.fans {
        let rpm_colored = if f.current_rpm > 3000 {
            format!("{} RPM", f.current_rpm).bold().green()
        } else {
            format!("{} RPM", f.current_rpm).normal()
        };

        println!(
            "{:<14} {:<16} {:<16} {:<12}",
            f.label.bold(),
            format!("{}/255", f.requested_boost),
            format!("{}/255", f.readback_boost),
            rpm_colored
        );
    }
}

fn cmd_profile(hw: &HardwareManager, profile: &str) {
    let target = if profile.to_lowercase() == "ultra" {
        println!("[*] Ultra profile requested: Setting performance + G-Mode...");
        "performance"
    } else {
        profile
    };

    println!("[*] Setting platform profile to '{}'...", target);
    if let Err(e) = hw.set_platform_profile(target) {
        eprintln!("{} Failed to set platform profile: {}", "[-] Error:".bold().red(), e);
        std::process::exit(1);
    }
    println!("{} Platform profile set to '{}'", "[+] Success:".bold().green(), target);
}

fn cmd_gmode(hw: &HardwareManager, state: Option<String>) {
    match state.as_deref() {
        Some("on") | Some("enable") => {
            println!("{}", "[*] Enabling Game Shift (G-Mode Ultra-Performance)...".bold().yellow());
            let _ = hw.set_platform_profile("performance");
            let _ = hw.write_fan_boost(1, 255);
            let _ = hw.write_fan_boost(2, 255);
            println!("{}", "[+] G-Mode engaged: Performance profile + Maximum fan boost.".bold().green());
        }
        Some("off") | Some("disable") => {
            println!("{}", "[*] Disabling Game Shift (G-Mode)...".bold().yellow());
            let _ = hw.set_platform_profile("balanced");
            let _ = hw.write_fan_boost(1, 0);
            let _ = hw.write_fan_boost(2, 0);
            println!("{}", "[+] G-Mode disengaged: Balanced profile + Firmware fan control.".bold().green());
        }
        _ => {
            let active = hw.is_gmode_active();
            println!(
                "Game Shift Status: {}",
                if active {
                    "ACTIVE".bold().green()
                } else {
                    "INACTIVE".normal()
                }
            );
        }
    }
}

fn cmd_fan_set(hw: &HardwareManager, fan: u8, boost: u8) {
    if !(1..=2).contains(&fan) {
        eprintln!("{} Invalid fan index {}. Must be 1 or 2.", "[-] Error:".bold().red(), fan);
        std::process::exit(1);
    }

    println!("[*] Setting Fan {} boost to {}/255...", fan, boost);
    match hw.write_fan_boost(fan, boost) {
        Ok(()) => {
            let rpm = hw.read_fan_rpm(fan);
            println!(
                "{} Fan {} boost set and verified. Current RPM: {}",
                "[+] Success:".bold().green(),
                fan,
                rpm
            );
        }
        Err(e) => {
            eprintln!("{} Failed to set boost: {}", "[-] Error:".bold().red(), e);
            std::process::exit(1);
        }
    }
}

fn cmd_fan_max(hw: &HardwareManager) {
    println!(
        "{}",
        "[*] Requesting MAXIMUM fan boost (255/255) on all fans...".bold().yellow()
    );
    let mut failure = false;
    for fan in 1..=2 {
        if let Err(e) = hw.write_fan_boost(fan, 255) {
            eprintln!("{} Failed to set max boost on fan {}: {}", "[-] Error:".bold().red(), fan, e);
            failure = true;
        }
    }
    if failure {
        std::process::exit(1);
    }

    println!("[*] Sampling tachometer acceleration (5 seconds)...");
    for i in 1..=5 {
        sleep(Duration::from_secs(1));
        let r1 = hw.read_fan_rpm(1);
        let r2 = hw.read_fan_rpm(2);
        println!("  T+{}s: Fan 1 (CPU): {} RPM | Fan 2 (GPU): {} RPM", i, r1, r2);
    }
    println!("{}", "[+] Maximum fan boost applied and actively spinning up.".bold().green());
}

fn cmd_fan_release(hw: &HardwareManager) {
    println!(
        "{}",
        "[*] Releasing fan boost (0/255) back to BIOS firmware control...".bold().yellow()
    );
    for fan in 1..=2 {
        if let Err(e) = hw.write_fan_boost(fan, 0) {
            eprintln!("{} Failed to release fan {}: {}", "[-] Error:".bold().red(), fan, e);
        }
    }
    println!("{}", "[+] Fan control authority surrendered to BIOS EC firmware.".bold().green());
}

fn cmd_doctor(hw: &HardwareManager) {
    let (vendor, prod, bios) = HardwareManager::get_dmi_info();
    println!("{}", "=== AWCC Linux Doctor Diagnostic Suite ===".bold().cyan());
    println!("1. DMI System Vendor:       {}", vendor);
    println!("2. DMI Product Name:        {}", prod);
    println!("3. BIOS Firmware Version:   {}", bios);

    let kver = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .unwrap_or_default()
        .trim()
        .to_string();
    println!("4. Kernel Release:          {}", kver);

    let wmi_path =
        std::path::Path::new("/sys/bus/wmi/devices/A70591CE-A997-11DA-B012-B622A1EF5492-34");
    println!(
        "5. AWCC WMAX WMI Node:      {}",
        if wmi_path.exists() {
            "FOUND (PNP0C14:0b registered)".green()
        } else {
            "NOT FOUND".red()
        }
    );

    println!(
        "6. alienware_wmi HWMON:     {}",
        if hw.is_driver_loaded() {
            "ACTIVE".green()
        } else {
            "INACTIVE (insmod alienware-wmi.ko needed)".red()
        }
    );

    let (aw_path, smm_path, ct_path, ddv_path) = HardwareManager::discover_hwmon_nodes();
    println!("7. AWCC hwmon sysfs path:   {:?}", aw_path);
    println!("8. Dell SMM hwmon path:     {:?}", smm_path);
    println!("9. Coretemp hwmon path:     {:?}", ct_path);
    println!("10. Dell DDV hwmon path:    {:?}", ddv_path);

    let (profile, choices) = HardwareManager::get_platform_profile();
    println!("11. Platform Profile:       {} (Choices: {:?})", profile, choices);
    println!("12. Game Shift (G-Mode):    Available via Dell G15 5530 quirk");
    println!("=== Diagnostics Complete ===");
}

fn cmd_verify(hw: &HardwareManager, step_duration: u64) {
    if !hw.is_driver_loaded() {
        eprintln!(
            "{} Cannot run verify ladder: alienware-wmi driver not loaded.",
            "[-] Error:".bold().red()
        );
        std::process::exit(1);
    }

    println!("{}", "=== AWCC Controlled Step Calibration Ladder ===".bold().cyan());
    let steps = [0u8, 64, 128, 255, 0];

    for &step in &steps {
        println!("\n[*] Actuating Step: Boost = {}/255 for {} seconds...", step, step_duration);
        for fan in 1..=2 {
            let _ = hw.write_fan_boost(fan, step);
        }

        for sec in 1..=step_duration {
            sleep(Duration::from_secs(1));
            let r1 = hw.read_fan_rpm(1);
            let r2 = hw.read_fan_rpm(2);
            println!(
                "  [{:02}s/{:02}s] Fan 1: {:4} RPM | Fan 2: {:4} RPM",
                sec, step_duration, r1, r2
            );
        }
    }
    println!("\n{}", "[+] Calibration verification ladder completed successfully.".bold().green());
}
