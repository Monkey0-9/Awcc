use awcc_core::capabilities::{FanControlCapability, VerificationTier};
use awcc_core::HardwareManager;
use clap::{Parser, Subcommand};
use colored::*;
use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, sleep};
use std::time::Duration;

#[derive(Parser)]
#[command(name = "awccctl")]
#[command(author = "AWCC Linux Team")]
#[command(version = "0.3.0")]
#[command(about = "Universal Linux Hardware Performance & Thermal Control Utility", long_about = None)]
struct Cli {
    /// Custom sysfs root path (for diagnostics & testing)
    #[arg(long, global = true)]
    sysfs_root: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Detect hardware interface, active provider, and vendor DMI
    Detect,

    /// Show current hardware capabilities, tiers, and supported features
    Capabilities {
        /// Output formatted JSON
        #[arg(long)]
        json: bool,
    },

    /// Show current fan telemetry, thermals, and platform profile
    Status {
        /// Output formatted JSON
        #[arg(long)]
        json: bool,

        /// Continuously watch and update telemetry in real time
        #[arg(short, long)]
        watch: bool,

        /// Refresh interval in seconds for watch mode
        #[arg(short, long, default_value_t = 1.0)]
        interval: f64,
    },

    /// Dedicated real-time hardware telemetry tracking and monitoring console
    #[command(alias = "monitor")]
    Track {
        /// Output formatted JSON stream
        #[arg(long)]
        json: bool,

        /// Refresh interval in seconds
        #[arg(short, long, default_value_t = 1.0)]
        interval: f64,

        /// Single-shot snapshot
        #[arg(short, long, alias = "snapshot")]
        once: bool,

        /// CLI table tracking mode (default)
        #[arg(short, long)]
        cli: bool,
    },

    /// Set platform profile (e.g. performance, balanced, quiet, cool, custom)
    Profile {
        /// Profile name
        name: String,

        /// Continuously monitor live hardware performance, thermals, and fan speeds until stopped
        #[arg(short, long, alias = "watch")]
        monitor: bool,

        /// Refresh interval in seconds for monitor mode
        #[arg(short, long, default_value_t = 1.0)]
        interval: f64,
    },

    /// Toggle or query Dell Game Shift / Ultra Performance G-Mode
    Gmode {
        /// Optional state (on/off/toggle)
        state: Option<String>,

        /// Continuously monitor live hardware performance, thermals, and fan speeds until stopped
        #[arg(short, long, alias = "watch")]
        monitor: bool,

        /// Refresh interval in seconds for monitor mode
        #[arg(short, long, default_value_t = 1.0)]
        interval: f64,
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
        /// Continuously monitor live hardware performance, thermals, and fan speeds until stopped
        #[arg(short, long, alias = "watch")]
        monitor: bool,
    },
    /// Request maximum fan boost on all fans
    Max {
        /// Continuously monitor live hardware performance, thermals, and fan speeds until stopped
        #[arg(short, long, alias = "watch")]
        monitor: bool,
    },
    /// Release fan boost back to BIOS/firmware control
    Release {
        /// Continuously monitor live hardware performance, thermals, and fan speeds until stopped
        #[arg(short, long, alias = "watch")]
        monitor: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let hw = match cli.sysfs_root {
        Some(root) => HardwareManager::with_sysfs_root(root),
        None => HardwareManager::new(),
    };

    match cli.command {
        Commands::Detect => cmd_detect(&hw),
        Commands::Capabilities { json } => cmd_capabilities(&hw, json),
        Commands::Status {
            json,
            watch,
            interval,
        } => cmd_status(&hw, json, watch, interval),
        Commands::Track {
            json,
            interval,
            once,
            cli: _,
        } => cmd_track(&hw, json, interval, once),
        Commands::Profile {
            name,
            monitor,
            interval,
        } => cmd_profile(&hw, &name, monitor, interval),
        Commands::Gmode {
            state,
            monitor,
            interval,
        } => cmd_gmode(&hw, state, monitor, interval),
        Commands::Fan { command } => match command {
            FanCommands::Set {
                fan,
                boost,
                monitor,
            } => cmd_fan_set(&hw, fan, boost, monitor),
            FanCommands::Max { monitor } => cmd_fan_max(&hw, monitor),
            FanCommands::Release { monitor } => cmd_fan_release(&hw, monitor),
        },
        Commands::Doctor => cmd_doctor(&hw),
        Commands::Verify { step_duration } => cmd_verify(&hw, step_duration),
    }
}

fn cmd_capabilities(hw: &HardwareManager, json: bool) {
    let caps = hw.capabilities();
    if json {
        println!("{}", serde_json::to_string_pretty(&caps).unwrap());
        return;
    }

    println!(
        "{}",
        "=== Universal Linux Hardware Capabilities ==="
            .bold()
            .cyan()
    );
    println!(
        "  DMI Vendor:           {}",
        caps.vendor.to_string().green()
    );
    println!("  DMI Product:          {}", caps.product.green());
    println!("  Chassis Type:         {:?}", caps.chassis);
    println!(
        "  Verification Tier:    {}",
        match caps.tier {
            VerificationTier::Tier1PhysicallyVerified => caps.tier.to_string().bold().green(),
            VerificationTier::Tier2CommunitySupported => caps.tier.to_string().bold().yellow(),
            VerificationTier::Tier3GenericLinuxFallback => caps.tier.to_string().bold().cyan(),
            VerificationTier::Tier4Unsupported => caps.tier.to_string().bold().red(),
        }
    );
    println!(
        "  Active Provider:      {}",
        caps.active_provider_name.cyan()
    );
    println!(
        "  Fan Control:          {}",
        match &caps.fan_control {
            FanControlCapability::BoostOffset { max_boost } =>
                format!("Boost Offset (0-{} ladder)", max_boost).green(),
            FanControlCapability::StandardPwm { min, max } =>
                format!("Standard Linux HWMON PWM ({}-{})", min, max).green(),
            FanControlCapability::DiscreteLevels { levels } =>
                format!("Discrete Levels [{}]", levels.join(", ")).green(),
            FanControlCapability::None =>
                "Firmware Automated (Manual override unsupported)".yellow(),
        }
    );
    println!(
        "  Platform Profile:     {}",
        match &caps.platform_profile {
            awcc_core::capabilities::PlatformProfileCapability::Supported {
                provider,
                current,
                choices,
            } => format!(
                "Supported (Provider: {}, Current: {}, Available: [{}])",
                provider,
                current,
                choices.join(", ")
            )
            .green(),
            awcc_core::capabilities::PlatformProfileCapability::NotSupported =>
                "Not Supported".yellow(),
        }
    );
    println!(
        "  Game Shift (G-Mode):  {}",
        if caps.has_game_shift {
            "Supported (Dell Alienware WMAX)".green()
        } else {
            "Not Available on this platform".dimmed()
        }
    );
    println!(
        "  GPU Telemetry:        {}",
        if caps.has_gpu_telemetry {
            "Available (Discrete GPU sensors detected)".green()
        } else {
            "None or Integrated Only".dimmed()
        }
    );
    println!(
        "  Motherboard VRM Temp: {}",
        if caps.has_vrm_telemetry {
            "Available".green()
        } else {
            "Not Exposed".dimmed()
        }
    );
    println!(
        "  Ambient Air Temp:     {}",
        if caps.has_ambient_telemetry {
            "Available".green()
        } else {
            "Not Exposed".dimmed()
        }
    );
    println!(
        "  Secure Boot Lockdown: {}",
        if caps.secure_boot_active {
            "Active (Kernel lockdown enforced)".green()
        } else {
            "Inactive / Disabled".yellow()
        }
    );
}

fn cmd_detect(hw: &HardwareManager) {
    let caps = hw.capabilities();
    let (vendor, prod, bios) = hw.get_dmi_info();
    println!(
        "{}",
        "=== Universal Hardware Interface Detection ==="
            .bold()
            .cyan()
    );
    println!("  DMI Vendor:       {}", vendor.green());
    println!("  DMI Product:      {}", prod.green());
    println!("  BIOS Version:     {}", bios.green());
    println!("  Chassis Type:     {:?}", caps.chassis);
    println!(
        "  Hardware Tier:    {}",
        match caps.tier {
            VerificationTier::Tier1PhysicallyVerified => caps.tier.to_string().bold().green(),
            VerificationTier::Tier2CommunitySupported => caps.tier.to_string().bold().yellow(),
            VerificationTier::Tier3GenericLinuxFallback => caps.tier.to_string().bold().cyan(),
            VerificationTier::Tier4Unsupported => caps.tier.to_string().bold().red(),
        }
    );
    println!("  Active Provider:  {}", caps.active_provider_name.cyan());
    println!(
        "  Driver Loaded:    {}",
        if hw.is_driver_loaded() {
            "YES (alienware_wmi active)".green()
        } else if caps.active_provider_name != "dell_awcc" {
            format!("YES (Provider: {} active)", caps.active_provider_name).green()
        } else {
            "NO (vendor kernel module not loaded)".yellow()
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

fn render_status_snapshot(hw: &HardwareManager, json: bool) {
    render_status_snapshot_internal(hw, json, false, 1.0);
}

fn render_status_snapshot_internal(hw: &HardwareManager, json: bool, is_live: bool, interval: f64) {
    let tele = hw.get_telemetry();
    if json {
        println!("{}", serde_json::to_string_pretty(&tele).unwrap());
        return;
    }

    let caps = hw.capabilities();
    println!(
        "{}",
        "=========================================================================="
            .bold()
            .cyan()
    );
    println!(
        "  {} {}",
        "AWCC Linux:".bold().cyan(),
        "Universal Hardware Performance & Thermal Telemetry".bold()
    );
    println!(
        "  Target Hardware:  {} {} ({:?})",
        caps.vendor.to_string().green(),
        caps.product.green(),
        caps.chassis
    );
    println!(
        "  Hardware Tier:    {} | Provider: {}",
        caps.tier.to_string().bold(),
        caps.active_provider_name.cyan()
    );
    println!(
        "  Platform Profile: {} (Available: [{}])",
        tele.active_platform_profile.bold().yellow(),
        tele.platform_profile_choices.join(", ")
    );
    if caps.has_game_shift {
        println!("  Game Shift:       {}", tele.gmode_status);
    }
    println!(
        "{}",
        "--------------------------------------------------------------------------".dimmed()
    );

    // 1. Silicon Computation & Load
    println!(
        "{}",
        " [ SILICON COMPUTATION & PERFORMANCE ]".bold().yellow()
    );
    if let Some(ref cpu) = tele.cpu {
        let usage_str = cpu
            .usage_percent
            .map(|u| format!("{:5.1}%", u))
            .unwrap_or_else(|| " N/A".to_string());
        let cur_mhz = cpu
            .cur_freq_mhz
            .map(|f| format!("{:.0} MHz", f))
            .unwrap_or_else(|| "N/A".to_string());
        let max_mhz = cpu
            .max_freq_mhz
            .map(|f| format!("{:.0} MHz", f))
            .unwrap_or_else(|| "N/A".to_string());
        let rapl_str = cpu
            .package_power_watts
            .map(|w| format!("{:.1}W", w))
            .unwrap_or_else(|| "Protected (Root-Only)".dimmed().to_string());
        let gov_str = cpu.scaling_governor.as_deref().unwrap_or("unknown");
        let epp_str = cpu.energy_perf_preference.as_deref().unwrap_or("unknown");

        println!(
            "  CPU:  {:<16} Load: {} | Clock: {} (Max: {})",
            cpu.model_name.bold(),
            usage_str.bold().green(),
            cur_mhz.cyan(),
            max_mhz.dimmed()
        );
        println!(
            "        Package Power: {:<12} | Governor: {:<10} | EPP: {}",
            rapl_str.yellow(),
            gov_str.cyan(),
            epp_str.cyan()
        );
    }

    if let Some(ref gpu) = tele.gpu {
        let util_str = gpu
            .utilization_gpu_percent
            .map(|u| format!("{:5.1}%", u))
            .unwrap_or_else(|| " N/A".to_string());
        let core_clk = gpu
            .core_clock_mhz
            .map(|c| format!("{} MHz", c))
            .unwrap_or_else(|| "N/A".to_string());
        let mem_clk = gpu
            .memory_clock_mhz
            .map(|m| format!("{} MHz", m))
            .unwrap_or_else(|| "N/A".to_string());
        let vram_used = gpu.memory_used_mb.unwrap_or(0);
        let vram_total = gpu.memory_total_mb.unwrap_or(0);
        let vram_pct = if vram_total > 0 {
            (vram_used as f32 / vram_total as f32) * 100.0
        } else {
            0.0
        };
        let pwr_str = gpu
            .power_draw_watts
            .map(|w| format!("{:.1}W", w))
            .unwrap_or_else(|| "Idle/D3".to_string());
        let limit_str = gpu
            .power_limit_watts
            .map(|l| format!("{:.0}W TGP", l))
            .unwrap_or_else(|| "N/A".to_string());

        let model = gpu.model_name.as_deref().unwrap_or("GPU");
        println!(
            "  GPU:  {:<16} Load: {} | Core: {} | Mem: {}",
            model.bold(),
            util_str.bold().green(),
            core_clk.cyan(),
            mem_clk.cyan()
        );
        println!(
            "        VRAM: {:>4} / {:>4} MB ({:4.1}%) | Power: {} / {}",
            vram_used,
            vram_total,
            vram_pct,
            pwr_str.yellow(),
            limit_str.dimmed()
        );
    }

    // 2. System Memory & Power Delivery
    println!(
        "{}",
        "--------------------------------------------------------------------------".dimmed()
    );
    println!("{}", " [ SYSTEM MEMORY & POWER DELIVERY ]".bold().cyan());
    if let Some(ref mem) = tele.memory {
        let ram_temp_str = match (tele.thermals.ram_temp, tele.thermals.ram_dimm2_temp) {
            (Some(t1), Some(t2)) => format!(" • {:.0}°C/{:.0}°C", t1, t2),
            (Some(t1), None) => format!(" • {:.0}°C", t1),
            _ => String::new(),
        };
        println!(
            "  RAM:  {:>5} / {:>5} MB ({:5.1}%{}) | Swap: {:>5} / {:>5} MB",
            mem.ram_used_mb,
            mem.ram_total_mb,
            mem.ram_used_percent,
            ram_temp_str,
            mem.swap_used_mb,
            mem.swap_total_mb
        );
    }
    if let Some(ref pwr) = tele.power {
        let ac_str = if pwr.ac_online {
            "ONLINE".bold().green()
        } else {
            "DISCONNECTED".bold().yellow()
        };
        let bat_str = pwr
            .battery_percent
            .map(|b| format!("{:.0}%", b))
            .unwrap_or_else(|| "N/A".to_string());
        let bat_stat = pwr.battery_status.as_deref().unwrap_or("Unknown");
        let rate_str = pwr
            .charge_rate_watts
            .map(|r| format!(" ({:+.1}W)", r))
            .unwrap_or_default();

        println!(
            "  PWR:  AC Delivery: {} | Battery: {} [{}{}]",
            ac_str,
            bat_str.cyan(),
            bat_stat,
            rate_str
        );
    }

    // 3. Thermal Sensor Matrix
    println!(
        "{}",
        "--------------------------------------------------------------------------".dimmed()
    );
    println!("{}", " [ THERMAL SENSOR MATRIX ]".bold().green());
    let fmt_temp = |opt: Option<f32>| -> colored::ColoredString {
        match opt {
            Some(t) if t >= 85.0 => format!("{:5.1}°C", t).bold().red(),
            Some(t) if t >= 70.0 => format!("{:5.1}°C", t).bold().yellow(),
            Some(t) => format!("{:5.1}°C", t).bold().green(),
            None => "  N/A".dimmed(),
        }
    };

    println!(
        "  CPU Package Temp:   {}    Motherboard VRM:    {}",
        fmt_temp(tele.thermals.cpu_pkg_temp),
        fmt_temp(tele.thermals.vrm_temp)
    );
    println!(
        "  CPU Core Max Temp:  {}    Ambient Air Temp:   {}",
        fmt_temp(tele.thermals.cpu_core_max_temp),
        fmt_temp(tele.thermals.ambient_temp)
    );
    println!("  GPU Temperature:    {}", fmt_temp(tele.thermals.gpu_temp));

    if tele.thermals.ram_dimm2_temp.is_some() {
        println!(
            "  RAM SODIMM 1 Temp:  {}    RAM SODIMM 2 Temp:  {}",
            fmt_temp(tele.thermals.ram_temp),
            fmt_temp(tele.thermals.ram_dimm2_temp)
        );
        if tele.thermals.charger_temp.is_some() {
            println!(
                "  Power Charger VRM:  {}",
                fmt_temp(tele.thermals.charger_temp)
            );
        }
    } else if tele.thermals.ram_temp.is_some() || tele.thermals.charger_temp.is_some() {
        println!(
            "  Memory SODIMM:      {}    Power Charger VRM:  {}",
            fmt_temp(tele.thermals.ram_temp),
            fmt_temp(tele.thermals.charger_temp)
        );
    }

    // 4. Fans & Tachometers
    println!(
        "{}",
        "--------------------------------------------------------------------------".dimmed()
    );
    println!(
        "{}",
        " [ DUAL HIGH-PRESSURE COOLING TURBINES ]".bold().blue()
    );
    if tele.fans.is_empty() {
        println!(
            "  Fan Control: {}",
            "Firmware automated / no tachometer sensors exposed".dimmed()
        );
    } else {
        println!(
            "{:<16} {:<18} {:<18} {:<16} {:<14}",
            "FAN", "REQUESTED BOOST", "READBACK BOOST", "MEASURED RPM", "RATED CEILING"
        );
        println!("{:-<84}", "");

        for f in &tele.fans {
            let label_str = format!("{:<16}", f.label).bold().to_string();
            let req_str = format!("{:<18}", format!("{}/255", f.requested_boost));

            let readback_raw = match f.readback_boost {
                Some(b) => format!("{}/255", b),
                None => "Unavailable".to_string(),
            };
            let readback_padded = format!("{:<18}", readback_raw);
            let readback_str = if f.readback_boost.is_none() {
                readback_padded.dimmed().to_string()
            } else {
                readback_padded
            };

            let rpm_raw = match f.current_rpm {
                Some(rpm) => format!("{} RPM", rpm),
                None => "Unavailable".to_string(),
            };
            let rpm_padded = format!("{:<16}", rpm_raw);
            let rpm_str = match f.current_rpm {
                Some(rpm) if rpm > 3000 => rpm_padded.bold().green().to_string(),
                Some(_) => rpm_padded,
                None => rpm_padded.dimmed().to_string(),
            };

            let max_raw = match f.reported_max_rpm {
                Some(max) => format!("{} RPM", max),
                None => "5,029 RPM".to_string(),
            };
            let max_padded = format!("{:<14}", max_raw);
            let max_str = if f.reported_max_rpm.is_none() {
                max_padded.dimmed().to_string()
            } else {
                max_padded
            };

            println!(
                "{} {} {} {} {}",
                label_str, req_str, readback_str, rpm_str, max_str
            );
        }
    }
    if is_live {
        println!(
            "  {}",
            format!(
                "[ Live Monitoring Active • Refresh: {:.1}s • Press Ctrl+C to stop ]",
                interval
            )
            .bold()
            .yellow()
        );
    }
    println!(
        "{}",
        "=========================================================================="
            .bold()
            .cyan()
    );
}

fn run_live_watch_loop(hw: &HardwareManager, json: bool, interval: f64) {
    let interval_dur = Duration::from_secs_f64(interval.max(0.2));
    let term = Arc::new(AtomicBool::new(false));
    let mut signals = Signals::new(TERM_SIGNALS).expect("Failed to register signal handlers");

    let term_clone = Arc::clone(&term);
    thread::spawn(move || {
        if signals.forever().next().is_some() {
            term_clone.store(true, Ordering::SeqCst);
        }
    });

    if !json {
        // Hide cursor and clear screen once initially
        print!("\x1b[?25l\x1b[2J\x1b[H");
        io::stdout().flush().ok();
    }

    while !term.load(Ordering::SeqCst) {
        if json {
            let tele = hw.get_telemetry();
            println!("{}", serde_json::to_string(&tele).unwrap());
            io::stdout().flush().ok();
        } else {
            print!("\x1b[H");
            render_status_snapshot_internal(hw, false, true, interval);
            print!("\x1b[J");
            io::stdout().flush().ok();
        }
        sleep(interval_dur);
    }

    if !json {
        // Restore cursor and clear formatting on clean exit
        println!("\x1b[?25h\x1b[0m");
        println!("{}", "[+] Live monitoring stopped.".green().bold());
        io::stdout().flush().ok();
    }
}

fn cmd_status(hw: &HardwareManager, json: bool, watch: bool, interval: f64) {
    if !watch {
        render_status_snapshot(hw, json);
        return;
    }
    run_live_watch_loop(hw, json, interval);
}

fn cmd_track(hw: &HardwareManager, json: bool, interval: f64, once: bool) {
    if once {
        render_status_snapshot(hw, json);
        return;
    }
    run_live_watch_loop(hw, json, interval);
}

fn cmd_profile(hw: &HardwareManager, profile: &str, monitor: bool, interval: f64) {
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

    if monitor {
        println!("[*] Entering live hardware monitoring (Press Ctrl+C to stop)...");
        sleep(Duration::from_millis(300));
        run_live_watch_loop(hw, false, interval);
    }
}

fn cmd_gmode(hw: &HardwareManager, state: Option<String>, monitor: bool, interval: f64) {
    let caps = hw.capabilities();
    if !caps.has_game_shift && caps.active_provider_name != "dell_awcc" {
        eprintln!(
            "{} Game Shift is a Dell Alienware specific feature not supported on {}.\n    Use 'awccctl profile performance' for maximum performance mode on this platform.",
            "[-] Error:".bold().red(),
            caps.vendor
        );
        std::process::exit(1);
    }

    let effective_state = match state.as_deref() {
        Some("toggle") => {
            let (cur_prof, _, _) = hw.get_platform_profile();
            let fans = hw.discover_fans();
            let all_max = !fans.is_empty()
                && fans
                    .iter()
                    .all(|f| f.readback_boost.is_some_and(|b| b >= 200));
            if cur_prof == "performance" && all_max {
                "off"
            } else {
                "on"
            }
        }
        Some(s) => s,
        None => "query",
    };

    match effective_state {
        "on" | "enable" => {
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
            let fans = hw.discover_fans();
            let mut failed = false;
            for fan in &fans {
                if let Err(e) = hw.write_fan_boost(fan.fan_index, 255) {
                    eprintln!(
                        "{} Failed to set boost on fan {}: {}",
                        "[-] Error:".bold().red(),
                        fan.fan_index,
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
        "off" | "disable" => {
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
            let fans = hw.discover_fans();
            let mut failed = false;
            for fan in &fans {
                if let Err(e) = hw.write_fan_boost(fan.fan_index, 0) {
                    eprintln!(
                        "{} Failed to release fan {}: {}",
                        "[-] Error:".bold().red(),
                        fan.fan_index,
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

    if monitor {
        println!("[*] Entering live hardware monitoring (Press Ctrl+C to stop)...");
        sleep(Duration::from_millis(300));
        run_live_watch_loop(hw, false, interval);
    }
}

fn cmd_fan_set(hw: &HardwareManager, fan: u8, boost: u8, monitor: bool) {
    if !(1..=6).contains(&fan) {
        eprintln!(
            "{} Invalid fan index {}. Controllable fans are 1 to 6.",
            "[-] Error:".bold().red(),
            fan
        );
        std::process::exit(2);
    }

    let caps = hw.capabilities();
    if matches!(caps.fan_control, FanControlCapability::None) {
        eprintln!(
            "{} Manual fan control is not supported on this hardware (Vendor: {}, Tier: {}).\n    Automatic firmware thermal management is active.\n    Use 'awccctl profile <name>' to adjust thermal/power balance.",
            "[-] Error:".bold().red(),
            caps.vendor,
            caps.tier
        );
        std::process::exit(1);
    }

    println!("[*] Setting Fan {} boost to {}/255...", fan, boost);
    match hw.set_fan_level(fan, boost) {
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

    if monitor {
        println!("[*] Entering live hardware monitoring (Press Ctrl+C to stop)...");
        sleep(Duration::from_millis(300));
        run_live_watch_loop(hw, false, 1.0);
    }
}

fn cmd_fan_max(hw: &HardwareManager, monitor: bool) {
    let caps = hw.capabilities();
    if matches!(caps.fan_control, FanControlCapability::None) {
        eprintln!(
            "{} Maximum fan boost is not supported on this hardware (Vendor: {}, Tier: {}).\n    Automatic firmware thermal management is active.\n    Use 'awccctl profile performance' to request highest performance mode.",
            "[-] Error:".bold().red(),
            caps.vendor,
            caps.tier
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "[*] Requesting MAXIMUM fan boost on all available fans..."
            .bold()
            .yellow()
    );

    if let Err(e) = hw.set_max_fans() {
        eprintln!(
            "{} Failed to set max fans: {}",
            "[-] Error:".bold().red(),
            e
        );
        std::process::exit(1);
    }

    println!("[*] Sampling tachometer response (3 seconds)...");
    for i in 1..=3 {
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
        "[+] Maximum fan boost confirmed by active provider."
            .bold()
            .green()
    );

    if monitor {
        println!("[*] Entering live hardware monitoring (Press Ctrl+C to stop)...");
        sleep(Duration::from_millis(300));
        run_live_watch_loop(hw, false, 1.0);
    }
}

fn cmd_fan_release(hw: &HardwareManager, monitor: bool) {
    println!(
        "{}",
        "[*] Releasing fan boost back to BIOS firmware control..."
            .bold()
            .yellow()
    );

    if let Err(e) = hw.release_fans() {
        eprintln!(
            "{} Failed to release fans: {}",
            "[-] Error:".bold().red(),
            e
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "[+] Fan control authority surrendered to BIOS EC firmware."
            .bold()
            .green()
    );

    if monitor {
        println!("[*] Entering live hardware monitoring (Press Ctrl+C to stop)...");
        sleep(Duration::from_millis(300));
        run_live_watch_loop(hw, false, 1.0);
    }
}

fn check_sysfs_permissions(root: &Path) -> (bool, Vec<String>) {
    let mut violations = Vec::new();
    let sysfs_dir = root.join("sys");
    if !sysfs_dir.exists() {
        return (true, violations);
    }

    // Inspect critical sysfs directories if they exist
    let targets = [
        root.join("sys/class/platform-profile"),
        root.join("sys/class/hwmon"),
        root.join("sys/devices/platform"),
    ];

    for target in &targets {
        if let Ok(entries) = fs::read_dir(target) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Ok(metadata) = p.metadata() {
                    let mode = metadata.permissions().mode();
                    if (mode & 0o002) != 0 {
                        violations.push(format!(
                            "{}: world-writable (mode {:o})",
                            p.display(),
                            mode
                        ));
                    }
                }
            }
        }
    }

    (violations.is_empty(), violations)
}

fn cmd_doctor(hw: &HardwareManager) {
    let caps = hw.capabilities();
    let (vendor, prod, bios) = hw.get_dmi_info();
    println!(
        "{}",
        "=== Universal Linux Hardware Doctor Diagnostic Suite ==="
            .bold()
            .cyan()
    );
    println!("1. DMI System Vendor:       {}", vendor);
    println!("2. DMI Product Name:        {}", prod);
    println!("3. BIOS Firmware Version:   {}", bios);
    println!("4. Chassis Type:            {:?}", caps.chassis);
    println!(
        "5. Hardware Tier:           {}",
        caps.tier.to_string().bold()
    );
    println!(
        "6. Active Provider:         {}",
        caps.active_provider_name.cyan()
    );

    let os_release = fs::read_to_string(hw.sysfs_root().join("etc/os-release"))
        .or_else(|_| fs::read_to_string("/etc/os-release"))
        .unwrap_or_default();
    let distro_name = os_release
        .lines()
        .find(|l| l.starts_with("PRETTY_NAME="))
        .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"'))
        .unwrap_or("Unknown Linux Distribution");
    println!("7. Host OS Distribution:    {}", distro_name.green());

    let kver = fs::read_to_string(hw.sysfs_root().join("proc/sys/kernel/osrelease"))
        .or_else(|_| fs::read_to_string("/proc/sys/kernel/osrelease"))
        .unwrap_or_default()
        .trim()
        .to_string();
    println!("8. Kernel Release:          {}", kver);

    let lockdown = fs::read_to_string(hw.sysfs_root().join("sys/kernel/security/lockdown"))
        .or_else(|_| fs::read_to_string("/sys/kernel/security/lockdown"))
        .unwrap_or_default();
    println!(
        "9. Secure Boot Status:      {}",
        if lockdown.contains("[integrity]") || lockdown.contains("[confidentiality]") {
            "ACTIVE (Lockdown enabled)".green()
        } else if lockdown.contains("[none]") {
            "DISABLED (Lockdown none)".yellow()
        } else {
            "UNCONFINED / Securityfs not mounted".dimmed()
        }
    );

    let (perm_ok, violations) = check_sysfs_permissions(hw.sysfs_root());
    println!(
        "10. Sysfs Permission Audit: {}",
        if perm_ok {
            "PASSED (Zero world-writable nodes detected)".green()
        } else {
            format!("FAILED: {} violations found", violations.len()).red()
        }
    );
    for v in violations {
        println!("    - {}", v.red());
    }

    let (profile, choices, provider) = hw.get_platform_profile();
    println!(
        "11. Platform Profile:       {} (Provider: {:?}, Choices: {:?})",
        profile, provider, choices
    );
    println!("12. Fan Control Capability: {:?}", caps.fan_control);
    println!("13. Game Shift (G-Mode):    {}", hw.check_gmode_status());

    // Low-Level Hardware Assembly Diagnostics (x86_64 CPUID & RDTSC)
    let hw_feat = awcc_core::hw::cpuid::probe_cpu_features();
    if !hw_feat.vendor_id.is_empty() {
        println!(
            "14. CPUID Assembly Probe:   {} [Family 0x{:X}, Model 0x{:X}, Stepping {}]",
            if !hw_feat.brand_string.is_empty() {
                &hw_feat.brand_string
            } else {
                &hw_feat.vendor_id
            },
            hw_feat.family,
            hw_feat.model,
            hw_feat.stepping
        );
        println!(
            "    - Thermal & Power Feats: DTS: {}, Turbo: {}, HWP (Speed Shift): {}, EPP: {}",
            if hw_feat.digital_thermal_sensor {
                "YES".green()
            } else {
                "NO".dimmed()
            },
            if hw_feat.turbo_boost {
                "YES".green()
            } else {
                "NO".dimmed()
            },
            if hw_feat.hwp_supported {
                "YES".green()
            } else {
                "NO".dimmed()
            },
            if hw_feat.hwp_epp_supported {
                "YES".green()
            } else {
                "NO".dimmed()
            },
        );
    }

    let bus_lat_ns = hw.measure_sysfs_bus_latency();
    let tsc_mhz = awcc_core::hw::rdtsc::get_or_calibrate_tsc_freq() / 1_000_000;
    println!(
        "15. Hardware Timer (RDTSC):  Calibrated TSC: {} MHz | ACPI Bus Latency: {}",
        tsc_mhz.to_string().cyan(),
        if bus_lat_ns < 100_000 {
            format!("{} ns (High Performance)", bus_lat_ns).green()
        } else {
            format!("{} ns (Standard ACPI)", bus_lat_ns).yellow()
        }
    );

    println!("=== Diagnostics Complete ===");
}

fn cmd_verify(hw: &HardwareManager, step_duration: u64) {
    let caps = hw.capabilities();
    if matches!(caps.fan_control, FanControlCapability::None) {
        eprintln!(
            "{} Cannot run calibration ladder: manual fan control is unsupported on this hardware (Vendor: {}, Tier: {}).",
            "[-] Error:".bold().red(),
            caps.vendor,
            caps.tier
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "=== Universal Controlled Step Calibration Ladder ==="
            .bold()
            .cyan()
    );
    let steps = [0u8, 64, 128, 255, 0];

    for &step in &steps {
        println!(
            "\n[*] Actuating Step: Level = {}/255 for {} seconds...",
            step, step_duration
        );
        for fan in 1..=2 {
            if let Err(e) = hw.set_fan_level(fan, step) {
                eprintln!("[-] Fan {}: {}", fan, e);
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
