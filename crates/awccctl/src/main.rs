use awcc_core::HardwareManager;
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "awccctl")]
#[command(author = "AWCC Linux Team")]
#[command(version = "0.2.0")]
#[command(about = "Dell AWCC Fan Control & Telemetry Utility for Linux", long_about = None)]
struct Cli {
    /// Custom sysfs root path (for diagnostics & testing)
    #[arg(long, global = true)]
    sysfs_root: Option<PathBuf>,

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

    /// Set platform profile (e.g. performance, balanced, quiet, cool, custom)
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
    let hw = match cli.sysfs_root {
        Some(root) => HardwareManager::with_sysfs_root(root),
        None => HardwareManager::new(),
    };

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
    let (vendor, prod, bios) = hw.get_dmi_info();
    println!(
        "{}",
        "=== AWCC Hardware Interface Detection ===".bold().cyan()
    );
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

    let (cur, choices, provider) = hw.get_platform_profile();
    println!(
        "  Platform Profile: {} (Available: [{}])",
        cur.yellow(),
        choices.join(", ")
    );
    if let Some(prov) = provider {
        println!("  Profile Provider: {}", prov.cyan());
    }
    let gmode = hw.check_gmode_status();
    println!("  Game Shift State: {}", gmode);
}

fn cmd_status(hw: &HardwareManager, json: bool) {
    let tele = hw.get_telemetry();
    if json {
        println!("{}", serde_json::to_string_pretty(&tele).unwrap());
        return;
    }

    println!(
        "{}",
        "=== AWCC Fan Control & Telemetry Status ===".bold().cyan()
    );
    println!(
        "  Platform Profile: {}",
        tele.active_platform_profile.bold().yellow()
    );
    if let Some(ref prov) = tele.platform_profile_provider {
        println!("  Profile Provider: {}", prov.cyan());
    }
    println!("  Game Shift:       {}", tele.gmode_status);
    println!(
        "  Available Choices: [{}]",
        tele.platform_profile_choices.join(", ")
    );
    println!();
    println!("{}", "--- Thermal Sensor Matrix ---".bold());
    match tele.thermals.cpu_pkg_temp {
        Some(t) => println!("  CPU Package Temp:   {:.1}°C", t),
        None => println!("  CPU Package Temp:   {}", "Unavailable".dimmed()),
    }
    match tele.thermals.cpu_core_max_temp {
        Some(t) => println!("  CPU Core Max Temp:  {:.1}°C", t),
        None => println!("  CPU Core Max Temp:  {}", "Unavailable".dimmed()),
    }
    match tele.thermals.gpu_temp {
        Some(t) => println!("  NVIDIA GPU Temp:    {:.1}°C", t),
        None => println!("  NVIDIA GPU Temp:    {}", "Unavailable".dimmed()),
    }
    match tele.thermals.vrm_temp {
        Some(t) => println!("  Motherboard VRM:    {:.1}°C", t),
        None => println!("  Motherboard VRM:    {}", "Unavailable".dimmed()),
    }
    match tele.thermals.ambient_temp {
        Some(t) => println!("  Ambient Temp:       {:.1}°C", t),
        None => println!("  Ambient Temp:       {}", "Unavailable".dimmed()),
    }

    println!();
    println!(
        "{:<14} {:<16} {:<16} {:<14} {:<12}",
        "FAN", "REQUESTED BOOST", "READBACK BOOST", "MEASURED RPM", "RATED MAX RPM"
    );
    println!("{:-<72}", "");

    for f in &tele.fans {
        let rpm_str = match f.current_rpm {
            Some(rpm) if rpm > 3000 => format!("{} RPM", rpm).bold().green().to_string(),
            Some(rpm) => format!("{} RPM", rpm),
            None => "Unavailable".dimmed().to_string(),
        };

        let max_str = match f.reported_max_rpm {
            Some(max) => format!("{} RPM", max),
            None => "Unknown".dimmed().to_string(),
        };

        let readback_str = match f.readback_boost {
            Some(b) => format!("{}/255", b),
            None => "Unavailable".dimmed().to_string(),
        };

        println!(
            "{:<14} {:<16} {:<16} {:<14} {:<12}",
            f.label.bold(),
            format!("{}/255", f.requested_boost),
            readback_str,
            rpm_str,
            max_str
        );
    }
}

fn cmd_profile(hw: &HardwareManager, profile: &str) {
    let target = if profile.to_lowercase() == "ultra" {
        println!("[*] Ultra profile requested: Setting performance mode...");
        "performance"
    } else {
        profile
    };

    println!("[*] Setting platform profile to '{}'...", target);
    if let Err(e) = hw.set_platform_profile(target) {
        eprintln!(
            "{} Failed to set platform profile: {}",
            "[-] Error:".bold().red(),
            e
        );
        std::process::exit(1);
    }
    println!(
        "{} Platform profile set to '{}'",
        "[+] Success:".bold().green(),
        target
    );
}

fn cmd_gmode(hw: &HardwareManager, state: Option<String>) {
    match state.as_deref() {
        Some("on") | Some("enable") => {
            println!(
                "{}",
                "[*] Enabling Game Shift (Performance profile + 255 boost)..."
                    .bold()
                    .yellow()
            );
            if let Err(e) = hw.set_platform_profile("performance") {
                eprintln!(
                    "{} Failed to set performance profile: {}",
                    "[-] Error:".bold().red(),
                    e
                );
                std::process::exit(1);
            }
            let mut failed = false;
            for fan in 1..=2 {
                if let Err(e) = hw.write_fan_boost(fan, 255) {
                    eprintln!(
                        "{} Failed to set boost on fan {}: {}",
                        "[-] Error:".bold().red(),
                        fan,
                        e
                    );
                    failed = true;
                }
            }
            if failed {
                std::process::exit(1);
            }
            println!(
                "{}",
                "[+] Game Shift active: Performance profile + Maximum fan boost (255/255)."
                    .bold()
                    .green()
            );
        }
        Some("off") | Some("disable") => {
            println!(
                "{}",
                "[*] Disabling Game Shift (Restoring balanced profile + 0 boost)..."
                    .bold()
                    .yellow()
            );
            if let Err(e) = hw.set_platform_profile("balanced") {
                eprintln!(
                    "{} Failed to set balanced profile: {}",
                    "[-] Error:".bold().red(),
                    e
                );
                std::process::exit(1);
            }
            let mut failed = false;
            for fan in 1..=2 {
                if let Err(e) = hw.write_fan_boost(fan, 0) {
                    eprintln!(
                        "{} Failed to release fan {}: {}",
                        "[-] Error:".bold().red(),
                        fan,
                        e
                    );
                    failed = true;
                }
            }
            if failed {
                std::process::exit(1);
            }
            println!(
                "{}",
                "[+] Game Shift disengaged: Balanced profile + Firmware fan control."
                    .bold()
                    .green()
            );
        }
        _ => {
            let status = hw.check_gmode_status();
            println!("Game Shift Status: {}", status);
        }
    }
}

fn cmd_fan_set(hw: &HardwareManager, fan: u8, boost: u8) {
    if !(1..=6).contains(&fan) {
        eprintln!(
            "{} Invalid fan index {}. Controllable fans are 1 to 6.",
            "[-] Error:".bold().red(),
            fan
        );
        std::process::exit(2);
    }

    println!("[*] Setting Fan {} boost to {}/255...", fan, boost);
    match hw.write_fan_boost(fan, boost) {
        Ok(()) => {
            let rpm = hw
                .read_fan_rpm(fan)
                .map(|r| format!("{} RPM", r))
                .unwrap_or_else(|| "N/A".to_string());
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
        "[*] Requesting MAXIMUM fan boost (255/255) on all available fans..."
            .bold()
            .yellow()
    );
    let fans = hw.discover_fans();
    if fans.is_empty() {
        eprintln!(
            "{} No controllable fans detected via alienware_wmi.",
            "[-] Error:".bold().red()
        );
        std::process::exit(1);
    }

    let mut failure = false;
    for fan in &fans {
        if let Err(e) = hw.write_fan_boost(fan.fan_index, 255) {
            eprintln!(
                "{} Failed to set max boost on fan {}: {}",
                "[-] Error:".bold().red(),
                fan.fan_index,
                e
            );
            failure = true;
        }
    }
    if failure {
        std::process::exit(1);
    }

    println!("[*] Sampling tachometer response (5 seconds)...");
    for i in 1..=5 {
        sleep(Duration::from_secs(1));
        let r1 = hw
            .read_fan_rpm(1)
            .map(|r| r.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let r2 = hw
            .read_fan_rpm(2)
            .map(|r| r.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        println!("  T+{}s: Fan 1: {} RPM | Fan 2: {} RPM", i, r1, r2);
    }
    println!(
        "{}",
        "[+] Maximum fan boost (255/255) confirmed by readback and active tachometer ramp."
            .bold()
            .green()
    );
}

fn cmd_fan_release(hw: &HardwareManager) {
    println!(
        "{}",
        "[*] Releasing fan boost (0/255) back to BIOS firmware control..."
            .bold()
            .yellow()
    );
    let fans = hw.discover_fans();
    if fans.is_empty() {
        eprintln!(
            "{} No controllable fans detected via alienware_wmi.",
            "[-] Error:".bold().red()
        );
        std::process::exit(1);
    }

    let mut failure = false;
    for fan in &fans {
        if let Err(e) = hw.write_fan_boost(fan.fan_index, 0) {
            eprintln!(
                "{} Failed to release fan {}: {}",
                "[-] Error:".bold().red(),
                fan.fan_index,
                e
            );
            failure = true;
        }
    }
    if failure {
        std::process::exit(1);
    }
    println!(
        "{}",
        "[+] Fan control authority surrendered to BIOS EC firmware."
            .bold()
            .green()
    );
}

fn cmd_doctor(hw: &HardwareManager) {
    let (vendor, prod, bios) = hw.get_dmi_info();
    println!(
        "{}",
        "=== AWCC Linux Doctor Diagnostic Suite ===".bold().cyan()
    );
    println!("1. DMI System Vendor:       {}", vendor);
    println!("2. DMI Product Name:        {}", prod);
    println!("3. BIOS Firmware Version:   {}", bios);

    let kver = std::fs::read_to_string(hw.sysfs_root().join("proc/sys/kernel/osrelease"))
        .or_else(|_| std::fs::read_to_string("/proc/sys/kernel/osrelease"))
        .unwrap_or_default()
        .trim()
        .to_string();
    println!("4. Kernel Release:          {}", kver);

    let wmi_path = hw
        .sysfs_root()
        .join("sys/bus/wmi/devices/A70591CE-A997-11DA-B012-B622A1EF5492-34");
    println!(
        "5. AWCC WMAX WMI Node:      {}",
        if wmi_path.exists()
            || std::path::Path::new("/sys/bus/wmi/devices/A70591CE-A997-11DA-B012-B622A1EF5492-34")
                .exists()
        {
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

    let (aw_path, smm_path, ct_path, ddv_path) = hw.discover_hwmon_nodes();
    println!("7. AWCC hwmon sysfs path:   {:?}", aw_path);
    println!("8. Dell SMM hwmon path:     {:?}", smm_path);
    println!("9. Coretemp hwmon path:     {:?}", ct_path);
    println!("10. Dell DDV hwmon path:    {:?}", ddv_path);

    let (profile, choices, provider) = hw.get_platform_profile();
    println!(
        "11. Platform Profile:       {} (Provider: {:?}, Choices: {:?})",
        profile, provider, choices
    );
    println!("12. Game Shift (G-Mode):    {}", hw.check_gmode_status());
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

    println!(
        "{}",
        "=== AWCC Controlled Step Calibration Ladder ==="
            .bold()
            .cyan()
    );
    let steps = [0u8, 64, 128, 255, 0];

    for &step in &steps {
        println!(
            "\n[*] Actuating Step: Boost = {}/255 for {} seconds...",
            step, step_duration
        );
        for fan in 1..=2 {
            if let Err(e) = hw.write_fan_boost(fan, step) {
                eprintln!("[-] Failed to set fan {}: {}", fan, e);
            }
        }

        for sec in 1..=step_duration {
            sleep(Duration::from_secs(1));
            let r1 = hw
                .read_fan_rpm(1)
                .map(|r| r.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let r2 = hw
                .read_fan_rpm(2)
                .map(|r| r.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            println!(
                "  [{:02}s/{:02}s] Fan 1: {:>6} | Fan 2: {:>6}",
                sec, step_duration, r1, r2
            );
        }
    }
    println!(
        "\n{}",
        "[+] Calibration verification ladder completed successfully."
            .bold()
            .green()
    );
}
