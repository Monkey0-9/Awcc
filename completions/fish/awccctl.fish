# Fish completion script for awccctl

complete -c awccctl -f

# Subcommands
complete -c awccctl -n "__fish_use_subcommand" -a detect -d "Detect hardware interface, active provider, and vendor DMI"
complete -c awccctl -n "__fish_use_subcommand" -a status -d "Query comprehensive hardware telemetry"
complete -c awccctl -n "__fish_use_subcommand" -a battery -d "Battery Saver (Quiet + 20% fan boost)"
complete -c awccctl -n "__fish_use_subcommand" -a quiet -d "Quiet profile (Acoustic silence + 40% fan boost)"
complete -c awccctl -n "__fish_use_subcommand" -a balanced -d "Balanced profile (Daily productivity + 60% fan boost)"
complete -c awccctl -n "__fish_use_subcommand" -a performance -d "Performance profile (High workload + 80% fan boost)"
complete -c awccctl -n "__fish_use_subcommand" -a ultra -d "Ultra profile (Maximum cooling & Game Shift + 100% fan boost)"
complete -c awccctl -n "__fish_use_subcommand" -a profile -d "Get or set platform performance profile"
complete -c awccctl -n "__fish_use_subcommand" -a fan -d "Manage fan speed and boost offset levels"
complete -c awccctl -n "__fish_use_subcommand" -a gmode -d "Actuate or query Game Shift (G-Mode)"
complete -c awccctl -n "__fish_use_subcommand" -a track -d "Stream continuous live hardware telemetry"
complete -c awccctl -n "__fish_use_subcommand" -a monitor -d "Alias for track command"
complete -c awccctl -n "__fish_use_subcommand" -a capabilities -d "Inspect hardware capabilities and provider detection"
complete -c awccctl -n "__fish_use_subcommand" -a doctor -d "Run system diagnostics, security audit, and CPUID/RDTSC checks"
complete -c awccctl -n "__fish_use_subcommand" -a verify -d "Run step calibration ladder across cooling fans"

# Options
complete -c awccctl -s h -l help -d "Show help information"
complete -c awccctl -s V -l version -d "Print version information"
complete -c awccctl -n "__fish_seen_subcommand_from detect status capabilities track monitor" -l json -d "Format output as structured JSON"
complete -c awccctl -n "__fish_seen_subcommand_from battery quiet balanced performance ultra" -s w -l watch -d "Watch and stream real-time telemetry after actuation"
complete -c awccctl -n "__fish_seen_subcommand_from battery quiet balanced performance ultra" -s m -l monitor -d "Stream real-time telemetry after actuation"
complete -c awccctl -n "__fish_seen_subcommand_from battery quiet balanced performance ultra" -l interval -d "Sampling interval in seconds"
complete -c awccctl -n "__fish_seen_subcommand_from track monitor" -l interval -d "Sampling interval in seconds"
complete -c awccctl -n "__fish_seen_subcommand_from track monitor" -l once -d "Output single telemetry snapshot and exit"
complete -c awccctl -n "__fish_seen_subcommand_from profile" -a "quiet balanced performance low-power custom" -d "Platform profile name"
complete -c awccctl -n "__fish_seen_subcommand_from gmode" -a "status on off toggle" -d "Game Shift action"
