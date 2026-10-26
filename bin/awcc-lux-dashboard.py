#!/usr/bin/env python3
"""
AWCC Linux Cybernetic Master Control & Telemetry Cockpit
Ultra-Luxurious Terminal UI & Real-Time Hardware Governor for Dell G15 5530
Target: Dell G15 5530 (i7-13650HX, RTX 3050 6GB) on RHEL 10.2
"""

import os
import sys
import time
import subprocess
import signal
import select
import re
import unicodedata
import shutil

# Strips ANSI escape codes to compute visible terminal width
ANSI_RE = re.compile(r'\x1b\[[0-9;]*[mGKHJ?]')

def char_width(c):
    if unicodedata.east_asian_width(c) in ('F', 'W'):
        return 2
    return 1

def str_display_width(s):
    clean = ANSI_RE.sub('', s)
    return sum(char_width(c) for c in clean)

def fit(text, width, align='left'):
    w = str_display_width(text)
    if w > width:
        tokens = re.split(r'(\x1b\[[0-9;]*[mGKHJ?])', text)
        result = []
        cur_w = 0
        for token in tokens:
            if not token:
                continue
            if token.startswith('\x1b'):
                result.append(token)
            else:
                for c in token:
                    cw = char_width(c)
                    if cur_w + cw > width:
                        break
                    result.append(c)
                    cur_w += cw
                if cur_w >= width:
                    break
        return ''.join(result) + RESET + (' ' * (width - cur_w))
    pad = width - w
    if align == 'right':
        return (' ' * pad) + text
    elif align == 'center':
        s1 = pad // 2
        s2 = pad - s1
        return (' ' * s1) + text + (' ' * s2)
    else:
        return text + (' ' * pad)

# TrueColor Cybernetic Alienware Palette
RESET         = '\033[0m'
BOLD          = '\033[1m'
DIM           = '\033[2m'

CYAN_ACCENT   = '\033[38;2;0;235;255m'
CYAN_BRIGHT   = '\033[1;38;2;0;255;230m'
NEON_GREEN    = '\033[1;38;2;0;255;170m'
ELECTRIC_PURP = '\033[1;38;2;180;100;255m'
SOLAR_AMBER   = '\033[1;38;2;255;190;0m'
CRIMSON_RED   = '\033[1;38;2;255;45;75m'
BLAZE_ORANGE  = '\033[1;38;2;255;120;0m'
MUTED_GRAY    = '\033[38;2;125;140;160m'
TEXT_DIM      = '\033[38;2;165;180;195m'
WHITE_BOLD    = '\033[1;38;2;255;255;255m'
TRACK_GRAY    = '\033[38;2;70;85;110m'

# Standard Box Dimensions: Locked at 78 cols / 74 inner / 12 gauge
# Guarantees mathematical alignment and zero line wrapping on standard 80x24 terminals
BOX_WIDTH = 78
INNER_WIDTH = 74
GAUGE_WIDTH = 12

def get_box_dimensions():
    return BOX_WIDTH, INNER_WIDTH, GAUGE_WIDTH

def make_row(content, inner_width=INNER_WIDTH):
    w = str_display_width(content)
    if w > inner_width:
        content = fit(content, inner_width)
        w = inner_width
    pad = max(0, inner_width - w)
    return f"{CYAN_ACCENT}║{RESET} {content}{' ' * pad} {CYAN_ACCENT}║{RESET}\033[K"

def align_split(left_text, right_text, target_width=INNER_WIDTH):
    lw = str_display_width(left_text)
    rw = str_display_width(right_text)
    spaces = max(1, target_width - lw - rw)
    return f"{left_text}{' ' * spaces}{right_text}"

def align_three(left, center, right, target_width=INNER_WIDTH):
    lw = str_display_width(left)
    cw = str_display_width(center)
    rw = str_display_width(right)
    rem = max(2, target_width - lw - cw - rw)
    s1 = rem // 2
    s2 = rem - s1
    return f"{left}{' ' * s1}{center}{' ' * s2}{right}"

def render_gauge(val, max_val, width=GAUGE_WIDTH, color_stops=None):
    if color_stops is None:
        color_stops = [
            (0.0, (0, 235, 255)),
            (0.4, (0, 255, 170)),
            (0.7, (255, 190, 0)),
            (1.0, (255, 45, 75))
        ]
    ratio = max(0.0, min(1.0, (val / max_val) if max_val > 0 else 0.0))
    filled = int(ratio * width)
    empty = width - filled

    chars = []
    for i in range(filled):
        t = i / max(1, width - 1)
        c = color_stops[-1][1]
        for s in range(len(color_stops) - 1):
            s1_t, s1_c = color_stops[s]
            s2_t, s2_c = color_stops[s + 1]
            if s1_t <= t <= s2_t:
                local_t = (t - s1_t) / (s2_t - s1_t) if s2_t > s1_t else 0.0
                c = (
                    int(s1_c[0] + (s2_c[0] - s1_c[0]) * local_t),
                    int(s1_c[1] + (s2_c[1] - s1_c[1]) * local_t),
                    int(s1_c[2] + (s2_c[2] - s1_c[2]) * local_t)
                )
                break
        chars.append(f"\033[38;2;{c[0]};{c[1]};{c[2]}m█")

    chars.append(f"{TRACK_GRAY}{'░' * empty}{RESET}")
    return "".join(chars)

def format_telemetry_line(label, gauge_str, val_str, mid_str, right_str, inner_width=INNER_WIDTH):
    l = fit(label, 14)
    v = fit(val_str, 7, 'right')
    m = fit(mid_str, 18)
    r = fit(right_str, 15)
    line = f"{l} {gauge_str} {v} {MUTED_GRAY}│{RESET} {m} {MUTED_GRAY}│{RESET} {r}"
    return make_row(line, inner_width)

def get_hwmon():
    for d in os.listdir("/sys/class/hwmon"):
        p = os.path.join("/sys/class/hwmon", d)
        name_path = os.path.join(p, "name")
        if os.path.isfile(name_path):
            try:
                with open(name_path, "r") as f:
                    if f.read().strip() == "alienware_wmi":
                        return p
            except Exception:
                pass
    return None

def get_cpu_info(prev_total, prev_idle):
    try:
        with open("/proc/stat", "r") as f:
            fields = [float(x) for x in f.readline().strip().split()[1:]]
        idle = fields[3] + fields[4]
        total = sum(fields)
        diff_total = total - prev_total
        diff_idle = idle - prev_idle
        usage = 0.0
        if diff_total > 0:
            usage = ((diff_total - diff_idle) / diff_total) * 100.0
    except Exception:
        total, idle, usage = 0.0, 0.0, 0.0

    mhz_list = []
    try:
        with open("/proc/cpuinfo", "r") as f:
            for line in f:
                if "cpu MHz" in line:
                    mhz_list.append(float(line.split(":")[1].strip()))
        avg_mhz = sum(mhz_list) / len(mhz_list) if mhz_list else 1600.0
    except Exception:
        avg_mhz = 1600.0

    temp = 45.0
    for h in ["hwmon7", "hwmon5", "hwmon4", "hwmon3"]:
        tp = f"/sys/class/hwmon/{h}/temp1_input"
        if os.path.isfile(tp):
            try:
                with open(tp, "r") as f:
                    temp = float(f.read().strip()) / 1000.0
                    break
            except Exception:
                pass

    return total, idle, usage, avg_mhz, temp

def get_gpu_info():
    try:
        cmd = [
            "nvidia-smi",
            "--query-gpu=utilization.gpu,temperature.gpu,clocks.current.graphics,clocks.current.memory,memory.used,memory.total,power.draw",
            "--format=csv,noheader,nounits"
        ]
        res = subprocess.run(cmd, capture_output=True, text=True, check=True)
        parts = [p.strip() for p in res.stdout.strip().split(",")]
        pwr = float(parts[6]) if len(parts) > 6 and parts[6] != "N/A" else 0.0
        if pwr > 150.0:
            pwr_str = "Idle/D3"
        else:
            pwr_str = f"{pwr:.1f}W"
        return {
            "util": float(parts[0]),
            "temp": float(parts[1]),
            "clk": int(parts[2]),
            "mem_clk": int(parts[3]),
            "vram_used": int(parts[4]),
            "vram_total": int(parts[5]),
            "pwr_str": pwr_str,
            "available": True
        }
    except Exception:
        return {
            "util": 0.0, "temp": 34.0, "clk": 210, "mem_clk": 405,
            "vram_used": 1, "vram_total": 6144, "pwr_str": "Idle/D3",
            "available": False
        }

def get_fans(aw_hwmon):
    b1, b2, r1, r2 = 0, 0, 0, 0
    if aw_hwmon and os.path.isdir(aw_hwmon):
        try:
            with open(os.path.join(aw_hwmon, "fan1_boost"), "r") as f: b1 = int(f.read().strip())
            with open(os.path.join(aw_hwmon, "fan2_boost"), "r") as f: b2 = int(f.read().strip())
            with open(os.path.join(aw_hwmon, "fan1_input"), "r") as f: r1 = int(f.read().strip())
            with open(os.path.join(aw_hwmon, "fan2_input"), "r") as f: r2 = int(f.read().strip())
        except Exception:
            pass
    if r1 == 0 or r2 == 0:
        for fallback in ["hwmon4"]:
            fb_p = f"/sys/class/hwmon/{fallback}"
            try:
                if r1 == 0 and os.path.isfile(f"{fb_p}/fan1_input"):
                    with open(f"{fb_p}/fan1_input", "r") as f: r1 = int(f.read().strip())
                if r2 == 0 and os.path.isfile(f"{fb_p}/fan2_input"):
                    with open(f"{fb_p}/fan2_input", "r") as f: r2 = int(f.read().strip())
            except Exception:
                pass
    return b1, b2, r1, r2

def get_power_info():
    ac_online = True
    bat_pct = 100
    bat_status = "Full"
    try:
        if os.path.isfile("/sys/class/power_supply/AC/online"):
            with open("/sys/class/power_supply/AC/online", "r") as f:
                ac_online = (f.read().strip() == "1")
        if os.path.isfile("/sys/class/power_supply/BAT0/capacity"):
            with open("/sys/class/power_supply/BAT0/capacity", "r") as f:
                bat_pct = int(f.read().strip())
        if os.path.isfile("/sys/class/power_supply/BAT0/status"):
            with open("/sys/class/power_supply/BAT0/status", "r") as f:
                bat_status = f.read().strip()
    except Exception:
        pass
    return ac_online, bat_pct, bat_status

MODE_DEFS = {
    1: {"name": "battery", "profile": "quiet",       "boost": 10,  "label": "BATTERY SAVER",    "desc": "10% Boost (~1,000 RPM)"},
    2: {"name": "quiet",   "profile": "quiet",       "boost": 20,  "label": "QUIET",            "desc": "20% Boost (~1,450 RPM)"},
    3: {"name": "balance", "profile": "balanced",    "boost": 40,  "label": "BALANCE",          "desc": "40% Boost (~1,980 RPM)"},
    4: {"name": "mid",     "profile": "performance", "boost": 65,  "label": "MID",              "desc": "65% Boost (~3,100 RPM)"},
    5: {"name": "high",    "profile": "performance", "boost": 85,  "label": "HIGH PERFORMANCE", "desc": "85% Boost (~4,200 RPM)"},
    6: {"name": "ultra",   "profile": "performance", "boost": 255, "label": "ULTRA / G-MODE",   "desc": "100% Boost (5,029 RPM)"},
}

def apply_mode_direct(mode_idx, aw_hwmon):
    cfg = MODE_DEFS.get(mode_idx)
    if not cfg:
        return

    # 1. Apply ACPI platform profile directly to kernel
    try:
        with open("/sys/firmware/acpi/platform_profile", "w") as f:
            f.write(cfg["profile"])
    except Exception:
        pass

    # 2. Apply fan boosts directly to hardware nodes
    if aw_hwmon and os.path.isdir(aw_hwmon):
        for b_name in ["fan1_boost", "fan2_boost"]:
            b_path = os.path.join(aw_hwmon, b_name)
            try:
                with open(b_path, "w") as f:
                    f.write(str(cfg["boost"]))
            except Exception:
                pass

    # 3. Synchronize Game Shift state file
    try:
        with open("/tmp/.awcc_gmode_state", "w") as f:
            f.write("ON\n" if cfg["boost"] >= 240 else "OFF\n")
    except Exception:
        pass

    # 4. Trigger non-blocking visual/audio feedback
    try:
        is_turbo = (cfg["boost"] >= 240)
        icon = "preferences-system-performance" if is_turbo else "preferences-system-power"
        sound = "service-login" if is_turbo else "dialog-information"
        subprocess.Popen(
            ["notify-send", "-u", "normal", "-t", "2000", "-i", icon,
             f"AWCC Mode: {cfg['label']}",
             f"Active Profile: {cfg['profile'].upper()} | {cfg['desc']}"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        )
        subprocess.Popen(
            ["canberra-gtk-play", "-i", sound],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        )
    except Exception:
        pass

def toggle_gameshift_direct(aw_hwmon):
    b1 = 0
    if aw_hwmon and os.path.isdir(aw_hwmon):
        try:
            with open(os.path.join(aw_hwmon, "fan1_boost"), "r") as f:
                b1 = int(f.read().strip())
        except Exception:
            pass
    if b1 >= 240:
        apply_mode_direct(3, aw_hwmon)
    else:
        apply_mode_direct(6, aw_hwmon)

def decode_key(text):
    if not text:
        return None
    # Function key F9 / Fn+F9
    if any(k in text for k in ['\x1b[20~', '\x1b[19~', '\x1b[21~', '\x1b[24~', '\x1b[20', '\x1b[19', '\x1b[21']):
        return 'F9'
    # Numpad & number row 1-6:
    if '1' in text or '\x1bOq' in text or '\x1b[4~' in text:
        return 1
    if '2' in text or '\x1bOr' in text:
        return 2
    if '3' in text or '\x1bOs' in text or '\x1b[6~' in text:
        return 3
    if '4' in text or '\x1bOt' in text:
        return 4
    if '5' in text or '\x1bOu' in text or '\x1b[E' in text or '\x1b[G' in text:
        return 5
    if '6' in text or '\x1bOv' in text:
        return 6
    if any(c in text for c in ['q', 'Q', 'x', 'X']) and not text.startswith('\x1b'):
        return 'QUIT'
    if ('r' in text or 'R' in text) and not text.startswith('\x1b'):
        return 'REFRESH'
    return None

def build_cockpit_frame(cpu_usage, cpu_mhz, cpu_temp, gpu, b1, b2, r1, r2, profile, mem_used, mem_total, ac_online, bat_pct, bat_status):
    box_width, inner_width, gauge_width = get_box_dimensions()
    gmode_active = (b1 >= 240)

    if b1 >= 240:
        mode_label = "ULTRA (100% Boost)"
        mode_color = CRIMSON_RED
    elif b1 >= 75:
        mode_label = "HIGH (85% Boost)"
        mode_color = BLAZE_ORANGE
    elif b1 >= 50:
        mode_label = "MID (65% Boost)"
        mode_color = SOLAR_AMBER
    elif b1 >= 30:
        mode_label = "BALANCE (40% Boost)"
        mode_color = CYAN_BRIGHT
    elif b1 >= 15:
        mode_label = "QUIET (20% Boost)"
        mode_color = NEON_GREEN
    elif b1 > 0:
        mode_label = "BATTERY (10% Boost)"
        mode_color = ELECTRIC_PURP
    else:
        mode_label = "PASSIVE (0% Boost)"
        mode_color = ELECTRIC_PURP

    if gmode_active:
        badge = f"\033[1;48;2;255;35;65m\033[38;2;255;255;255m  G-MODE EXTREME  \033[0m"
    else:
        badge = f"\033[48;2;32;42;58m\033[38;2;160;180;210m  ○ HYBRID QUIET  \033[0m"

    pwr_str = f"AC {bat_pct}% [ONLINE]" if ac_online else f"BAT {bat_pct}% [{bat_status.upper()}]"

    # Fan Gauges
    fan_stops = [
        (0.0, (0, 235, 255)),
        (0.5, (0, 255, 170)),
        (0.8, (255, 190, 0)),
        (1.0, (255, 45, 75))
    ]
    gauge_f1 = render_gauge(r1, 5029, gauge_width, fan_stops)
    gauge_f2 = render_gauge(r2, 5029, gauge_width, fan_stops)

    # CPU Temp
    t_cpu_col = CRIMSON_RED if cpu_temp >= 82 else (SOLAR_AMBER if cpu_temp >= 68 else NEON_GREEN)
    gauge_cpu = render_gauge(cpu_usage, 100, gauge_width)

    # GPU Temp
    g_temp = gpu["temp"]
    t_gpu_col = CRIMSON_RED if g_temp >= 78 else (SOLAR_AMBER if g_temp >= 62 else NEON_GREEN)
    gauge_gpu = render_gauge(gpu["util"], 100, gauge_width)

    # VRAM
    vram_pct = (gpu["vram_used"] / gpu["vram_total"] * 100) if gpu["vram_total"] > 0 else 0.0
    vram_used_gb = gpu["vram_used"] / 1024.0
    vram_total_gb = gpu["vram_total"] / 1024.0
    gauge_vram = render_gauge(vram_pct, 100, gauge_width, [
        (0.0, (180, 100, 255)),
        (0.7, (0, 235, 255)),
        (1.0, (255, 45, 75))
    ])

    # RAM
    ram_pct = (mem_used / mem_total * 100) if mem_total > 0 else 0.0
    gauge_ram = render_gauge(ram_pct, 100, gauge_width, [
        (0.0, (0, 235, 255)),
        (0.6, (180, 100, 255)),
        (1.0, (255, 45, 75))
    ])

    top = f"{CYAN_ACCENT}╔{'═' * (box_width - 2)}╗{RESET}\033[K"
    div_double = f"{CYAN_ACCENT}╠{'═' * (box_width - 2)}╣{RESET}\033[K"
    div_single = f"{CYAN_ACCENT}╟{'─' * (box_width - 2)}╢{RESET}\033[K"
    bot = f"{CYAN_ACCENT}╚{'═' * (box_width - 2)}╝{RESET}\033[K"

    rows = []
    rows.append(top)

    # Header Row
    hdr_left = f"{NEON_GREEN}▲ ALIENWARE COMMAND CENTER v2.4{RESET}"
    hdr_center = f"{WHITE_BOLD}DELL G15 5530{RESET}"
    hdr_right = f"{MUTED_GRAY}RHEL 10.2{RESET}"
    rows.append(make_row(align_three(hdr_left, hdr_center, hdr_right, inner_width), inner_width))
    rows.append(div_double)

    # Mode / Status Row
    stat_left = f"{MUTED_GRAY}MODE:{RESET} {mode_color}{mode_label}{RESET}"
    stat_right = f"{MUTED_GRAY}PWR:{RESET} {WHITE_BOLD}{pwr_str}{RESET}"
    rows.append(make_row(align_three(stat_left, badge, stat_right, inner_width), inner_width))
    rows.append(div_double)

    # Cooling Section
    cool_left = f"{NEON_GREEN}[ DUAL HIGH-PRESSURE COOLING TURBINES ]{RESET}"
    cool_right = f"{MUTED_GRAY}MAX CEILING: 5,029 RPM{RESET}"
    rows.append(make_row(align_split(cool_left, cool_right, inner_width), inner_width))

    def boost_pct_str(b):
        if b >= 240: return "100%"
        if b >= 75:  return " 85%"
        if b >= 50:  return " 65%"
        if b >= 30:  return " 40%"
        if b >= 15:  return " 20%"
        if b > 0:    return " 10%"
        return "  0%"

    f1_val = f"{WHITE_BOLD}{r1:4d}RPM{RESET}"
    f1_mid = f"Boost: {CYAN_BRIGHT}{b1:3d} ({boost_pct_str(b1)}){RESET}"
    f1_right = f"{MUTED_GRAY}Max: 5,029 RPM{RESET}"
    rows.append(format_telemetry_line(f"{WHITE_BOLD}FAN 1 (CPU){RESET}", gauge_f1, f1_val, f1_mid, f1_right, inner_width))

    f2_val = f"{WHITE_BOLD}{r2:4d}RPM{RESET}"
    f2_mid = f"Boost: {CYAN_BRIGHT}{b2:3d} ({boost_pct_str(b2)}){RESET}"
    f2_right = f"{MUTED_GRAY}Max: 5,029 RPM{RESET}"
    rows.append(format_telemetry_line(f"{WHITE_BOLD}FAN 2 (GPU){RESET}", gauge_f2, f2_val, f2_mid, f2_right, inner_width))
    rows.append(div_double)

    # Silicon Section
    silicon_left = f"{SOLAR_AMBER}[ SILICON COMPUTATION & THERMALS ]{RESET}"
    silicon_right = f"{MUTED_GRAY}ACPI WMAX: PNP0C14:0b{RESET}"
    rows.append(make_row(align_split(silicon_left, silicon_right, inner_width), inner_width))

    # CPU Row
    cpu_val = f"{SOLAR_AMBER}{cpu_usage:5.1f}%{RESET}"
    cpu_mid = f"Clock: {WHITE_BOLD}{cpu_mhz:4.0f} MHz{RESET}"
    cpu_right = f"{t_cpu_col}Temp: {cpu_temp:4.1f}°C{RESET}"
    rows.append(format_telemetry_line(f"{WHITE_BOLD}i7-13650HX CPU{RESET}", gauge_cpu, cpu_val, cpu_mid, cpu_right, inner_width))

    # GPU Row
    gpu_val = f"{NEON_GREEN}{gpu['util']:5.1f}%{RESET}"
    gpu_mid = f"Clock: {WHITE_BOLD}{gpu['clk']:4d} MHz{RESET}"
    gpu_right = f"{t_gpu_col}Temp: {g_temp:4.1f}°C{RESET}"
    rows.append(format_telemetry_line(f"{WHITE_BOLD}RTX 3050 GPU{RESET}", gauge_gpu, gpu_val, gpu_mid, gpu_right, inner_width))

    # VRAM Row
    vram_val = f"{ELECTRIC_PURP}{vram_pct:5.1f}%{RESET}"
    vram_mid = f"Used:  {WHITE_BOLD}{vram_used_gb:4.2f} GB{RESET}"
    vram_right = f"{MUTED_GRAY}Limit: {vram_total_gb:4.2f} GB{RESET}"
    rows.append(format_telemetry_line(f"{WHITE_BOLD}GDDR6 VRAM 6G{RESET}", gauge_vram, vram_val, vram_mid, vram_right, inner_width))

    # RAM Row
    ram_val = f"{CYAN_BRIGHT}{ram_pct:5.1f}%{RESET}"
    ram_mid = f"Used:  {WHITE_BOLD}{mem_used:4.1f} GB{RESET}"
    ram_right = f"{MUTED_GRAY}Total: {mem_total:4.1f} GB{RESET}"
    rows.append(format_telemetry_line(f"{WHITE_BOLD}SYSTEM DDR5{RESET}", gauge_ram, ram_val, ram_mid, ram_right, inner_width))
    rows.append(div_double)

    # Hotkeys and Controls Section
    hk_left = f"{ELECTRIC_PURP}HARDWARE HOTKEY:{RESET} {WHITE_BOLD}F9 / Fn+F9{RESET} (Game Shift Toggle)"
    hk_right = f"{MUTED_GRAY}HARDWARE TURBO{RESET}"
    rows.append(make_row(align_split(hk_left, hk_right, inner_width), inner_width))
    rows.append(div_single)

    # Dynamic mode button highlight in cockpit footer
    active_idx = 3
    if b1 >= 240:
        active_idx = 6
    elif b1 >= 75:
        active_idx = 5
    elif b1 >= 50:
        active_idx = 4
    elif b1 >= 30:
        active_idx = 3
    elif b1 >= 15:
        active_idx = 2
    else:
        active_idx = 1

    mode_defs_list = [
        (1, "Battery", "\033[1;97;45m"),
        (2, "Quiet",   "\033[1;30;42m"),
        (3, "Balance", "\033[1;30;46m"),
        (4, "Mid",     "\033[1;30;43m"),
        (5, "High",    "\033[1;97;41m"),
        (6, "Ultra",   "\033[1;97;41m"),
    ]
    mode_spans = []
    for idx, name, bg_col in mode_defs_list:
        if idx == active_idx:
            mode_spans.append(f"{bg_col} [{idx}] {name} \033[0m")
        else:
            mode_spans.append(f"{MUTED_GRAY}[{idx}] {name}{RESET}")
    modes_line = f"{WHITE_BOLD}MODES:{RESET}  " + " ".join(mode_spans)
    rows.append(make_row(modes_line, inner_width))

    acts_line = f"{CYAN_ACCENT}ACTIONS:{RESET} [F9] Game Shift Toggle  •  [1-6] Profiles  •  [Q] Exit Cockpit"
    rows.append(make_row(acts_line, inner_width))
    rows.append(bot)

    return "\n".join(rows) + "\033[J"

def run_snapshot():
    aw_hwmon = get_hwmon()
    _, _, cpu_usage, cpu_mhz, cpu_temp = get_cpu_info(0, 0)
    gpu = get_gpu_info()
    b1, b2, r1, r2 = get_fans(aw_hwmon)

    profile = "balanced"
    try:
        with open("/sys/firmware/acpi/platform_profile", "r") as f:
            profile = f.read().strip()
    except Exception:
        pass

    mem_used, mem_total = 6.4, 15.1
    try:
        with open("/proc/meminfo", "r") as f:
            lines = f.readlines()
            t = int(lines[0].split()[1]) / 1024 / 1024
            a = int(lines[2].split()[1]) / 1024 / 1024
            mem_total = t
            mem_used = t - a
    except Exception:
        pass

    ac_online, bat_pct, bat_status = get_power_info()
    frame = build_cockpit_frame(cpu_usage, cpu_mhz, cpu_temp, gpu, b1, b2, r1, r2, profile, mem_used, mem_total, ac_online, bat_pct, bat_status)
    print(frame)

def run_continuous():
    import termios
    import tty

    fd = sys.stdin.fileno() if sys.stdin.isatty() else None
    old_term = None
    if fd is not None:
        try:
            old_term = termios.tcgetattr(fd)
            tty.setcbreak(fd)
        except Exception:
            fd = None

    def cleanup():
        if fd is not None and old_term is not None:
            try:
                termios.tcsetattr(fd, termios.TCSADRAIN, old_term)
            except Exception:
                pass
        # Leave alternate screen buffer, restore cursor, reset attributes
        sys.stdout.write("\033[?1049l\033[?25h" + RESET + "\n")
        sys.stdout.flush()

    def sig_handler(sig, frame):
        cleanup()
        sys.exit(0)

    def on_winch(sig, frame):
        # Handle terminal resize: clear entire screen and reposition cursor
        sys.stdout.write("\033[2J\033[H")
        sys.stdout.flush()

    signal.signal(signal.SIGINT, sig_handler)
    signal.signal(signal.SIGTERM, sig_handler)
    signal.signal(signal.SIGWINCH, on_winch)

    # Enter alternate screen buffer (\033[?1049h), DISABLE alternate scroll mode (\033[?1007l),
    # hide cursor (\033[?25l), clear screen (\033[2J), home cursor (\033[H)
    sys.stdout.write("\033[?1049h\033[?1007l\033[?25l\033[2J\033[H")
    sys.stdout.flush()

    prev_total, prev_idle = 0.0, 0.0
    aw_hwmon = get_hwmon()
    needs_quick_redraw = False

    try:
        while True:
            # Event-driven non-blocking key wait: wakes up IMMEDIATELY (<1ms) on keypress,
            # or times out after 0.75s (or 0.08s for quick transition) to refresh telemetry
            timeout = 0.08 if needs_quick_redraw else 0.75
            needs_quick_redraw = False

            if fd is not None:
                r, _, _ = select.select([sys.stdin], [], [], timeout)
                if r:
                    try:
                        raw_bytes = os.read(fd, 1024)
                        text = raw_bytes.decode('utf-8', errors='ignore')
                    except Exception:
                        text = ""

                    action = decode_key(text)
                    if action == 'QUIT':
                        break
                    elif isinstance(action, int) and 1 <= action <= 6:
                        apply_mode_direct(action, aw_hwmon)
                        needs_quick_redraw = True
                    elif action == 'F9':
                        toggle_gameshift_direct(aw_hwmon)
                        needs_quick_redraw = True
                    elif action == 'REFRESH':
                        needs_quick_redraw = True
            else:
                time.sleep(0.75)

            prev_total, prev_idle, cpu_usage, cpu_mhz, cpu_temp = get_cpu_info(prev_total, prev_idle)
            gpu = get_gpu_info()
            b1, b2, r1, r2 = get_fans(aw_hwmon)

            profile = "balanced"
            try:
                with open("/sys/firmware/acpi/platform_profile", "r") as f:
                    profile = f.read().strip()
            except Exception:
                pass

            mem_used, mem_total = 6.4, 15.1
            try:
                with open("/proc/meminfo", "r") as f:
                    lines = f.readlines()
                    t = int(lines[0].split()[1]) / 1024 / 1024
                    a = int(lines[2].split()[1]) / 1024 / 1024
                    mem_total = t
                    mem_used = t - a
            except Exception:
                pass

            ac_online, bat_pct, bat_status = get_power_info()

            frame = build_cockpit_frame(cpu_usage, cpu_mhz, cpu_temp, gpu, b1, b2, r1, r2, profile, mem_used, mem_total, ac_online, bat_pct, bat_status)

            # Move cursor to home (1,1) and draw frame
            sys.stdout.write("\033[H" + frame)
            sys.stdout.flush()

    finally:
        cleanup()

if __name__ == "__main__":
    if "--snapshot" in sys.argv or "-s" in sys.argv:
        run_snapshot()
    else:
        run_continuous()
