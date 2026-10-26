#!/usr/bin/env python3
"""
AWCC Linux Cybernetic Master Control & Telemetry Cockpit
Multi-Vendor Hardware Governor, TrueColor Color-Graded Dashboard & Telemetry Matrix
Dell G15 / Alienware / Multi-Vendor Hardware Support on Linux / RHEL / Fedora / Ubuntu / Arch

Features:
- TrueColor (24-bit RGB) Color-Grading Theme Engine:
    * Alienware Cryo-Tech Pro (Default)
    * Cyberpunk 2077 Night City
    * Alienware Dark Core OLED
    * Solar Flare HUD
    * Matrix Emerald Phosphor
    * Vaporwave Sunset
- Live Interactive Theme Cycling with [C] or [T] Key
- Complete Hardware Telemetry Tracking:
    * CPU: Model, Logical Cores, Load %, Clocks, Temps, RAPL Package Power (Watts), Governor, EPP
    * GPU: Model, Load %, Graphics Clock, Memory Clock, Temp, VRAM Used/Total, Live Power Draw (Watts) vs TGP
    * Motherboard & Environment: VRM Temp, Ambient Air Temp, Charger Temp, SODIMM Temp
    * Cooling Turbines: RPM, Boost 0-255, Duty %, Max Ceiling (5,029 RPM), Acoustic Baseline
    * Memory & Power: RAM Breakdown, Swap, AC Status, Battery %, Charge/Discharge Watts
- Event-Driven Non-Blocking Keypad (<1ms response via select()): 1-6 Profiles, F9 Game Shift, C/T Themes
- Seamless SIGWINCH terminal resize adaptation without stippling or text wrapping
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

SYSFS_ROOT = "/"

def sysfs_path(rel):
    return os.path.join(SYSFS_ROOT, rel.lstrip("/"))

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
        return ''.join(result) + '\033[0m' + (' ' * (width - cur_w))
    pad = width - w
    if align == 'right':
        return (' ' * pad) + text
    elif align == 'center':
        s1 = pad // 2
        s2 = pad - s1
        return (' ' * s1) + text + (' ' * s2)
    else:
        return text + (' ' * pad)

# ANSI Basics & Terminal Capability Detection
RESET         = '\033[0m'
BOLD          = '\033[1m'
DIM           = '\033[2m'

def detect_color_support():
    term = os.environ.get("TERM", "").lower()
    colorterm = os.environ.get("COLORTERM", "").lower()
    if term == "dumb" or os.environ.get("NO_COLOR"):
        return False, False
    has_tc = colorterm in ("truecolor", "24bit") or "direct" in term or any(k in term for k in ("kitty", "alacritty", "foot", "wezterm", "xterm-ghostty"))
    has_256 = has_tc or "256color" in term or "xterm" in term
    return has_tc, has_256

HAS_TRUECOLOR, HAS_256_COLOR = detect_color_support()

def rgb_to_256(r, g, b):
    if r == g == b:
        if r < 8: return 16
        if r > 248: return 231
        return round(((r - 8) / 247) * 23) + 232
    return 16 + (36 * round(r / 255 * 5)) + (6 * round(g / 255 * 5)) + round(b / 255 * 5)

def ansi_rgb_fg(r, g, b, bold=False):
    prefix = "\033[1;" if bold else "\033["
    if HAS_TRUECOLOR:
        return f"{prefix}38;2;{r};{g};{b}m"
    elif HAS_256_COLOR:
        return f"{prefix}38;5;{rgb_to_256(r, g, b)}m"
    else:
        return f"{prefix}37m"

def ansi_rgb_bg(r, g, b):
    if HAS_TRUECOLOR:
        return f"\033[48;2;{r};{g};{b}m"
    elif HAS_256_COLOR:
        return f"\033[48;5;{rgb_to_256(r, g, b)}m"
    else:
        return "\033[40m"

WHITE_BOLD    = ansi_rgb_fg(255, 255, 255, bold=True)
MUTED_GRAY    = ansi_rgb_fg(125, 140, 160)
TEXT_DIM      = ansi_rgb_fg(165, 180, 195)

# --- TrueColor Color-Grading Theme Engine ---
class ColorTheme:
    def __init__(self, key, name, border_rgb, accent_rgb, title_rgb, track_rgb,
                 stops_cool, stops_cpu, stops_gpu, stops_vram, stops_ram,
                 badge_bg_rgb, badge_fg_rgb):
        self.key = key
        self.name = name
        self.border = ansi_rgb_fg(border_rgb[0], border_rgb[1], border_rgb[2])
        self.border_corner = ansi_rgb_fg(accent_rgb[0], accent_rgb[1], accent_rgb[2], bold=True)
        self.accent = ansi_rgb_fg(accent_rgb[0], accent_rgb[1], accent_rgb[2])
        self.accent_bright = ansi_rgb_fg(accent_rgb[0], accent_rgb[1], accent_rgb[2], bold=True)
        self.title = ansi_rgb_fg(title_rgb[0], title_rgb[1], title_rgb[2], bold=True)
        self.track = ansi_rgb_fg(track_rgb[0], track_rgb[1], track_rgb[2])
        self.stops_cool = stops_cool
        self.stops_cpu = stops_cpu
        self.stops_gpu = stops_gpu
        self.stops_vram = stops_vram
        self.stops_ram = stops_ram
        self.badge_bg = ansi_rgb_bg(badge_bg_rgb[0], badge_bg_rgb[1], badge_bg_rgb[2])
        self.badge_fg = ansi_rgb_fg(badge_fg_rgb[0], badge_fg_rgb[1], badge_fg_rgb[2], bold=True)

THEMES = {
    "cryo": ColorTheme(
        key="cryo",
        name="Alienware Cryo-Tech Pro",
        border_rgb=(34, 52, 74),
        accent_rgb=(0, 229, 255),
        title_rgb=(0, 255, 170),
        track_rgb=(25, 38, 54),
        stops_cool=[(0.0, (0, 220, 255)), (0.4, (0, 255, 180)), (0.7, (255, 190, 0)), (1.0, (255, 45, 75))],
        stops_cpu=[(0.0, (0, 230, 255)), (0.5, (0, 255, 170)), (0.8, (255, 180, 0)), (1.0, (255, 45, 75))],
        stops_gpu=[(0.0, (0, 255, 190)), (0.5, (0, 230, 255)), (0.8, (255, 160, 0)), (1.0, (255, 45, 75))],
        stops_vram=[(0.0, (170, 100, 255)), (0.6, (0, 230, 255)), (1.0, (255, 45, 75))],
        stops_ram=[(0.0, (0, 210, 255)), (0.6, (140, 110, 255)), (1.0, (255, 50, 80))],
        badge_bg_rgb=(13, 59, 66),
        badge_fg_rgb=(0, 242, 254),
    ),
    "cyberpunk": ColorTheme(
        key="cyberpunk",
        name="Cyberpunk 2077 Night City",
        border_rgb=(58, 42, 64),
        accent_rgb=(252, 238, 10),
        title_rgb=(255, 0, 85),
        track_rgb=(38, 28, 44),
        stops_cool=[(0.0, (0, 240, 255)), (0.5, (252, 238, 10)), (1.0, (255, 0, 85))],
        stops_cpu=[(0.0, (0, 240, 255)), (0.5, (252, 238, 10)), (1.0, (255, 0, 85))],
        stops_gpu=[(0.0, (0, 255, 170)), (0.5, (252, 238, 10)), (1.0, (255, 0, 85))],
        stops_vram=[(0.0, (255, 0, 180)), (0.5, (170, 0, 255)), (1.0, (255, 0, 85))],
        stops_ram=[(0.0, (0, 230, 255)), (0.5, (255, 0, 180)), (1.0, (252, 238, 10))],
        badge_bg_rgb=(60, 20, 40),
        badge_fg_rgb=(252, 238, 10),
    ),
    "darkcore": ColorTheme(
        key="darkcore",
        name="Alienware Dark Core OLED",
        border_rgb=(28, 38, 54),
        accent_rgb=(56, 189, 248),
        title_rgb=(248, 250, 252),
        track_rgb=(15, 23, 42),
        stops_cool=[(0.0, (56, 189, 248)), (0.6, (14, 165, 233)), (1.0, (244, 63, 94))],
        stops_cpu=[(0.0, (56, 189, 248)), (0.6, (99, 102, 241)), (1.0, (244, 63, 94))],
        stops_gpu=[(0.0, (45, 212, 191)), (0.6, (56, 189, 248)), (1.0, (244, 63, 94))],
        stops_vram=[(0.0, (168, 85, 247)), (0.6, (56, 189, 248)), (1.0, (244, 63, 94))],
        stops_ram=[(0.0, (56, 189, 248)), (0.6, (168, 85, 247)), (1.0, (244, 63, 94))],
        badge_bg_rgb=(15, 30, 50),
        badge_fg_rgb=(56, 189, 248),
    ),
    "solar": ColorTheme(
        key="solar",
        name="Solar Flare HUD",
        border_rgb=(65, 45, 30),
        accent_rgb=(255, 180, 0),
        title_rgb=(255, 215, 0),
        track_rgb=(42, 28, 18),
        stops_cool=[(0.0, (255, 215, 0)), (0.5, (255, 140, 0)), (1.0, (255, 40, 0))],
        stops_cpu=[(0.0, (255, 215, 0)), (0.5, (255, 140, 0)), (1.0, (255, 40, 0))],
        stops_gpu=[(0.0, (255, 200, 50)), (0.5, (255, 130, 0)), (1.0, (255, 40, 0))],
        stops_vram=[(0.0, (255, 140, 0)), (0.6, (255, 80, 0)), (1.0, (255, 0, 50))],
        stops_ram=[(0.0, (255, 200, 0)), (0.6, (255, 110, 0)), (1.0, (255, 40, 0))],
        badge_bg_rgb=(50, 30, 10),
        badge_fg_rgb=(255, 215, 0),
    ),
    "matrix": ColorTheme(
        key="matrix",
        name="Matrix Emerald Phosphor",
        border_rgb=(20, 55, 30),
        accent_rgb=(0, 255, 120),
        title_rgb=(0, 255, 160),
        track_rgb=(12, 32, 18),
        stops_cool=[(0.0, (0, 255, 140)), (0.6, (50, 220, 90)), (1.0, (200, 255, 50))],
        stops_cpu=[(0.0, (0, 255, 140)), (0.6, (50, 220, 90)), (1.0, (200, 255, 50))],
        stops_gpu=[(0.0, (0, 255, 120)), (0.6, (50, 220, 90)), (1.0, (200, 255, 50))],
        stops_vram=[(0.0, (0, 220, 160)), (0.6, (0, 255, 120)), (1.0, (180, 255, 40))],
        stops_ram=[(0.0, (0, 255, 130)), (0.6, (40, 200, 90)), (1.0, (180, 255, 40))],
        badge_bg_rgb=(10, 45, 20),
        badge_fg_rgb=(0, 255, 140),
    ),
    "vaporwave": ColorTheme(
        key="vaporwave",
        name="Vaporwave Sunset",
        border_rgb=(55, 30, 75),
        accent_rgb=(255, 113, 206),
        title_rgb=(1, 205, 254),
        track_rgb=(35, 18, 50),
        stops_cool=[(0.0, (1, 205, 254)), (0.5, (185, 103, 255)), (1.0, (255, 113, 206))],
        stops_cpu=[(0.0, (1, 205, 254)), (0.5, (185, 103, 255)), (1.0, (255, 113, 206))],
        stops_gpu=[(0.0, (5, 255, 161)), (0.5, (1, 205, 254)), (1.0, (255, 113, 206))],
        stops_vram=[(0.0, (185, 103, 255)), (0.6, (255, 113, 206)), (1.0, (255, 251, 150))],
        stops_ram=[(0.0, (1, 205, 254)), (0.6, (185, 103, 255)), (1.0, (255, 113, 206))],
        badge_bg_rgb=(50, 20, 65),
        badge_fg_rgb=(255, 113, 206),
    ),
}

THEME_ORDER = ["cryo", "cyberpunk", "darkcore", "solar", "matrix", "vaporwave"]
THEME_CONFIG_FILE = os.path.expanduser("~/.config/awcc/theme")

def load_persisted_theme():
    try:
        if os.path.isfile(THEME_CONFIG_FILE):
            with open(THEME_CONFIG_FILE, "r") as f:
                k = f.read().strip().lower()
                if k in THEMES:
                    return k
    except Exception:
        pass
    return "cryo"

def save_persisted_theme(k):
    try:
        os.makedirs(os.path.dirname(THEME_CONFIG_FILE), exist_ok=True)
        with open(THEME_CONFIG_FILE, "w") as f:
            f.write(k + "\n")
    except Exception:
        pass

CURRENT_THEME_KEY = load_persisted_theme()

def get_current_theme():
    return THEMES.get(CURRENT_THEME_KEY, THEMES["cryo"])

def cycle_next_theme():
    global CURRENT_THEME_KEY
    idx = THEME_ORDER.index(CURRENT_THEME_KEY) if CURRENT_THEME_KEY in THEME_ORDER else 0
    next_idx = (idx + 1) % len(THEME_ORDER)
    CURRENT_THEME_KEY = THEME_ORDER[next_idx]
    save_persisted_theme(CURRENT_THEME_KEY)
    return THEMES[CURRENT_THEME_KEY]

# Dynamic Terminal Geometry Engine
def get_box_dimensions():
    cols, rows = shutil.get_terminal_size(fallback=(80, 24))
    box_width = max(36, cols)
    inner_width = max(32, box_width - 4)
    if inner_width >= 120:
        gauge_width = min(32, inner_width // 4)
    elif inner_width >= 80:
        gauge_width = min(20, inner_width // 4)
    elif inner_width >= 60:
        gauge_width = 10
    else:
        gauge_width = 8
    return box_width, inner_width, gauge_width, rows

def make_row(content, inner_width, theme=None):
    t = theme if theme else get_current_theme()
    w = str_display_width(content)
    if w > inner_width:
        content = fit(content, inner_width)
        w = inner_width
    pad = max(0, inner_width - w)
    return f"{t.border}║{RESET} {content}{' ' * pad} {t.border}║{RESET}\033[K"

def align_split(left_text, right_text, target_width):
    lw = str_display_width(left_text)
    rw = str_display_width(right_text)
    spaces = max(1, target_width - lw - rw)
    return f"{left_text}{' ' * spaces}{right_text}"

def align_three(left, center, right, target_width):
    lw = str_display_width(left)
    cw = str_display_width(center)
    rw = str_display_width(right)
    rem = max(2, target_width - lw - cw - rw)
    s1 = rem // 2
    s2 = rem - s1
    return f"{left}{' ' * s1}{center}{' ' * s2}{right}"

def render_gauge(val, max_val, width, color_stops=None, theme=None):
    t = theme if theme else get_current_theme()
    if color_stops is None:
        color_stops = t.stops_cpu
    ratio = max(0.0, min(1.0, (val / max_val) if max_val > 0 else 0.0))
    filled = int(ratio * width)
    empty = width - filled

    chars = []
    for i in range(filled):
        pos_t = i / max(1, width - 1)
        c = color_stops[-1][1]
        for s in range(len(color_stops) - 1):
            s1_t, s1_c = color_stops[s]
            s2_t, s2_c = color_stops[s + 1]
            if s1_t <= pos_t <= s2_t:
                local_t = (pos_t - s1_t) / (s2_t - s1_t) if s2_t > s1_t else 0.0
                c = (
                    int(s1_c[0] + (s2_c[0] - s1_c[0]) * local_t),
                    int(s1_c[1] + (s2_c[1] - s1_c[1]) * local_t),
                    int(s1_c[2] + (s2_c[2] - s1_c[2]) * local_t)
                )
                break
        chars.append(f"{ansi_rgb_fg(c[0], c[1], c[2])}█")

    chars.append(f"{t.track}{'░' * empty}{RESET}")
    return "".join(chars)

def format_telemetry_line(label, gauge_str, val_str, mid_str, right_str, inner_width, gauge_width):
    if inner_width >= 80:
        lbl_w = 15
        val_w = 6
        fixed = lbl_w + 1 + gauge_width + 1 + val_w + 3 + 3
        rem = max(12, inner_width - fixed)
        right_w = 16
        mid_w = max(8, rem - right_w)
        l = fit(label, lbl_w)
        v = fit(val_str, val_w, 'right')
        m = fit(mid_str, mid_w)
        r = fit(right_str, right_w)
        line = f"{l} {gauge_str} {v} {MUTED_GRAY}│{RESET} {m} {MUTED_GRAY}│{RESET} {r}"
    elif inner_width >= 62:
        lbl_w = 14
        val_w = 7
        fixed = lbl_w + 1 + gauge_width + 1 + val_w + 3 + 3
        rem = max(10, inner_width - fixed)
        right_w = 15
        mid_w = max(8, rem - right_w)
        l = fit(label, lbl_w)
        v = fit(val_str, val_w, 'right')
        m = fit(mid_str, mid_w)
        r = fit(right_str, right_w)
        line = f"{l} {gauge_str} {v} {MUTED_GRAY}│{RESET} {m} {MUTED_GRAY}│{RESET} {r}"
    elif inner_width >= 48:
        lbl_w = 12
        val_w = 6
        rem = max(8, inner_width - lbl_w - 1 - gauge_width - 1 - val_w - 3)
        l = fit(label, lbl_w)
        v = fit(val_str, val_w, 'right')
        m = fit(mid_str, rem)
        line = f"{l} {gauge_str} {v} {MUTED_GRAY}│{RESET} {m}"
    else:
        lbl_w = min(12, max(8, inner_width - gauge_width - 8))
        l = fit(label, lbl_w)
        v = fit(val_str, 6, 'right')
        line = f"{l} {gauge_str} {v}"
    return make_row(line, inner_width)

def find_hwmon_by_name(target_name):
    hwmon_base = sysfs_path("sys/class/hwmon")
    if not os.path.isdir(hwmon_base):
        return None
    for d in os.listdir(hwmon_base):
        p = os.path.join(hwmon_base, d)
        name_path = os.path.join(p, "name")
        if os.path.isfile(name_path):
            try:
                with open(name_path, "r") as f:
                    if f.read().strip() == target_name:
                        return p
            except Exception:
                pass
    return None

def get_hwmon():
    return find_hwmon_by_name("alienware_wmi")

def get_dmi_info():
    vendor = "Generic"
    product = "Linux PC"
    try:
        v_path = sysfs_path("sys/devices/virtual/dmi/id/sys_vendor")
        if os.path.isfile(v_path):
            with open(v_path) as f:
                vendor = f.read().strip()
        p_path = sysfs_path("sys/devices/virtual/dmi/id/product_name")
        if os.path.isfile(p_path):
            with open(p_path) as f:
                product = f.read().strip()
    except Exception:
        pass

    # Clean and deduplicate: e.g. "Dell Inc." + "Dell G15 5530" -> "DELL G15 5530"
    v_clean = re.sub(r'(?i)\binc\.?|\bltd\.?|\bcorp\.?|\bcorporation\b', '', vendor).strip()
    if product.lower().startswith(v_clean.lower()):
        display_machine = product
    else:
        display_machine = f"{v_clean} {product}"
    return vendor, product, display_machine.strip()

def get_os_release():
    distro = "Linux"
    try:
        os_path = sysfs_path("etc/os-release")
        if not os.path.isfile(os_path):
            os_path = "/etc/os-release"
        if os.path.isfile(os_path):
            with open(os_path) as f:
                content = f.read()
                m_id = re.search(r'^ID=(.+)$', content, re.MULTILINE)
                m_ver = re.search(r'^VERSION_ID=(.+)$', content, re.MULTILINE)
                if m_id and m_ver:
                    distro_id = m_id.group(1).strip('"').upper()
                    ver = m_ver.group(1).strip('"')
                    return f"{distro_id} {ver}"
                for line in content.splitlines():
                    if line.startswith("PRETTY_NAME="):
                        distro = line.strip().split("=", 1)[1].strip('"')
                        distro = re.sub(r'\(.*?\)', '', distro).strip()
                        break
    except Exception:
        pass
    return distro

def get_cpu_model():
    model = "CPU"
    try:
        proc_cpu = sysfs_path("proc/cpuinfo")
        if not os.path.isfile(proc_cpu):
            proc_cpu = "/proc/cpuinfo"
        if os.path.isfile(proc_cpu):
            with open(proc_cpu) as f:
                for line in f:
                    if "model name" in line:
                        m = line.split(":", 1)[1].strip()
                        # Match specific Intel/AMD pattern: e.g. i7-13650HX, Ryzen 7 7840HS
                        for pat in [r'(i[3579]-\w+)', r'(Ryzen \d+ \w+)', r'(Ultra \d+ \w+)', r'(Xeon \w+)']:
                            match = re.search(pat, m)
                            if match:
                                return match.group(1)
                        cleaned = re.sub(r'\(R\)|\(TM\)|Processor|CPU|@.*', '', m).strip()
                        words = cleaned.split()
                        model = " ".join(words[:2]) if words else "CPU"
                        break
    except Exception:
        pass
    return model

def get_gpu_name():
    try:
        res = subprocess.run(["nvidia-smi", "--query-gpu=name", "--format=csv,noheader"], capture_output=True, text=True, timeout=1)
        if res.returncode == 0 and res.stdout.strip():
            name = res.stdout.strip().split("\n")[0]
            # e.g. "NVIDIA GeForce RTX 3050 6GB Laptop GPU" -> "RTX 3050 GPU"
            name = re.sub(r'NVIDIA GeForce | Laptop GPU| \d+GB', '', name).strip()
            return f"{name} GPU"
    except Exception:
        pass
    return "GPU ACCELERATOR"

def get_verification_tier(vendor, has_aw_hwmon):
    v = vendor.lower()
    if ("dell" in v or "alienware" in v) and has_aw_hwmon:
        return "[TIER 1]"
    elif any(k in v for k in ["asus", "lenovo", "hp", "framework", "system76", "dell", "alienware"]):
        return "[TIER 2]"
    return "[TIER 3]"

def get_platform_profile_path():
    class_dir = sysfs_path("sys/class/platform-profile")
    if os.path.isdir(class_dir):
        for entry in sorted(os.listdir(class_dir)):
            p = os.path.join(class_dir, entry)
            name_file = os.path.join(p, "name")
            if os.path.isfile(name_file):
                try:
                    with open(name_file, "r") as f:
                        if f.read().strip() == "alienware-wmi":
                            prof = os.path.join(p, "profile")
                            if os.path.isfile(prof):
                                return prof
                except Exception:
                    pass
        for entry in sorted(os.listdir(class_dir)):
            prof = os.path.join(class_dir, entry, "profile")
            if os.path.isfile(prof):
                return prof
    acpi_node = sysfs_path("sys/firmware/acpi/platform_profile")
    if os.path.isfile(acpi_node):
        return acpi_node
    return None

def write_sysfs_value(path, value):
    try:
        with open(path, "w") as f:
            f.write(str(value))
        return True, None
    except PermissionError:
        try:
            cmd = ["sudo", "tee", path]
            res = subprocess.run(
                cmd,
                input=str(value),
                text=True,
                capture_output=True,
                timeout=2
            )
            if res.returncode == 0:
                return True, None
            return False, f"sudo error ({res.returncode}): {res.stderr.strip()}"
        except Exception as e:
            return False, f"sudo failed: {e}"
    except Exception as e:
        return False, str(e)

def get_fan_max(aw_hwmon):
    m1, m2 = 4800, 4800
    if aw_hwmon and os.path.isdir(aw_hwmon):
        try:
            with open(os.path.join(aw_hwmon, "fan1_max"), "r") as f:
                m1 = int(f.read().strip())
        except Exception:
            pass
        try:
            with open(os.path.join(aw_hwmon, "fan2_max"), "r") as f:
                m2 = int(f.read().strip())
        except Exception:
            pass
    return m1, m2

def get_cpu_info(prev_total, prev_idle, prev_rapl_uj=0, prev_rapl_time=0.0):
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
    ct_p = find_hwmon_by_name("coretemp") or find_hwmon_by_name("alienware_wmi") or find_hwmon_by_name("dell_smm")
    if ct_p:
        tp = os.path.join(ct_p, "temp1_input")
        if os.path.isfile(tp):
            try:
                with open(tp, "r") as f:
                    temp = float(f.read().strip()) / 1000.0
            except Exception:
                pass

    rapl_watts = None
    cur_rapl_uj = 0
    cur_rapl_time = time.time()
    rapl_path = sysfs_path("sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj")
    if os.path.isfile(rapl_path):
        try:
            with open(rapl_path, "r") as f:
                cur_rapl_uj = int(f.read().strip())
            if prev_rapl_uj > 0 and cur_rapl_time > prev_rapl_time:
                diff_j = (cur_rapl_uj - prev_rapl_uj) / 1_000_000.0
                diff_t = cur_rapl_time - prev_rapl_time
                if diff_t > 0:
                    rapl_watts = diff_j / diff_t
        except Exception:
            pass

    return total, idle, usage, avg_mhz, temp, rapl_watts, cur_rapl_uj, cur_rapl_time

def get_gpu_info():
    try:
        cmd = [
            "nvidia-smi",
            "--query-gpu=utilization.gpu,temperature.gpu,clocks.current.graphics,clocks.current.memory,memory.used,memory.total,power.draw.average,power.draw.instant,enforced.power.limit",
            "--format=csv,noheader,nounits"
        ]
        res = subprocess.run(cmd, capture_output=True, text=True, check=True)
        parts = [p.strip() for p in res.stdout.strip().split(",")]

        pwr_avg = float(parts[6]) if len(parts) > 6 and parts[6] not in ("N/A", "[N/A]") else -1.0
        pwr_inst = float(parts[7]) if len(parts) > 7 and parts[7] not in ("N/A", "[N/A]") else -1.0
        pwr_limit = float(parts[8]) if len(parts) > 8 and parts[8] not in ("N/A", "[N/A]") else 80.0

        pwr = pwr_avg if (0.0 <= pwr_avg <= 150.0) else (pwr_inst if (0.0 <= pwr_inst <= 150.0) else -1.0)
        pwr_str = f"{pwr:.1f}W" if pwr >= 0 else "Idle/D3"

        return {
            "util": float(parts[0]),
            "temp": float(parts[1]),
            "clk": int(parts[2]),
            "mem_clk": int(parts[3]),
            "vram_used": int(parts[4]),
            "vram_total": int(parts[5]),
            "pwr_str": pwr_str,
            "pwr_val": pwr,
            "pwr_limit": pwr_limit,
            "available": True
        }
    except Exception:
        return {
            "util": 0.0, "temp": 34.0, "clk": 210, "mem_clk": 405,
            "vram_used": 1, "vram_total": 6144, "pwr_str": "Idle/D3",
            "pwr_val": 0.0, "pwr_limit": 80.0,
            "available": False
        }

def get_fans(aw_hwmon):
    b1, b2, r1, r2 = 0, 0, 0, 0
    fans = []
    if aw_hwmon and os.path.isdir(aw_hwmon):
        try:
            with open(os.path.join(aw_hwmon, "fan1_boost"), "r") as f: b1 = int(f.read().strip())
            with open(os.path.join(aw_hwmon, "fan2_boost"), "r") as f: b2 = int(f.read().strip())
            with open(os.path.join(aw_hwmon, "fan1_input"), "r") as f: r1 = int(f.read().strip())
            with open(os.path.join(aw_hwmon, "fan2_input"), "r") as f: r2 = int(f.read().strip())
        except Exception:
            pass
        m1, m2 = get_fan_max(aw_hwmon)
        fans.append({"index": 1, "label": "FAN 1 (CPU)", "boost": b1, "rpm": r1, "max": m1, "controllable": True})
        fans.append({"index": 2, "label": "FAN 2 (GPU)", "boost": b2, "rpm": r2, "max": m2, "controllable": True})
    else:
        hwmon_base = sysfs_path("sys/class/hwmon")
        if os.path.isdir(hwmon_base):
            for d in sorted(os.listdir(hwmon_base)):
                p = os.path.join(hwmon_base, d)
                if not os.path.isdir(p):
                    continue
                found_in_chip = False
                for i in range(1, 9):
                    inp = os.path.join(p, f"fan{i}_input")
                    if os.path.isfile(inp):
                        rpm = 0
                        try:
                            with open(inp) as f: rpm = int(f.read().strip())
                        except Exception: pass
                        lbl_f = os.path.join(p, f"fan{i}_label")
                        lbl = f"FAN {i}"
                        if os.path.isfile(lbl_f):
                            try:
                                with open(lbl_f) as f: lbl = f.read().strip().upper()
                            except Exception: pass
                        fans.append({"index": i, "label": lbl, "boost": 0, "rpm": rpm, "max": 4800, "controllable": False})
                        found_in_chip = True
                if found_in_chip:
                    break
        if not fans:
            fans.append({"index": 1, "label": "FAN 1", "boost": 0, "rpm": 0, "max": 4800, "controllable": False})

    return b1, b2, r1, r2, fans

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

def get_system_diagnostics():
    load1, load5, load15 = 0.0, 0.0, 0.0
    try:
        with open("/proc/loadavg") as f:
            p = f.read().split()
            load1, load5, load15 = float(p[0]), float(p[1]), float(p[2])
    except Exception:
        pass

    uptime_str = "0h 00m"
    try:
        with open("/proc/uptime") as f:
            sec = float(f.read().split()[0])
            h = int(sec // 3600)
            m = int((sec % 3600) // 60)
            uptime_str = f"{h}h {m:02d}m"
    except Exception:
        pass

    gov_str = "powersave"
    try:
        with open("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor") as f:
            gov_str = f.read().strip()
    except Exception:
        pass

    return load1, load5, load15, uptime_str, gov_str

def get_memory_breakdown():
    mem_avail_gb, swap_used_gb, swap_total_gb, mem_cached_gb = 0.0, 0.0, 0.0, 0.0
    try:
        with open("/proc/meminfo") as f:
            mem = {}
            for line in f:
                parts = line.split(":")
                if len(parts) == 2:
                    mem[parts[0].strip()] = int(parts[1].split()[0])
            mem_avail_gb = mem.get("MemAvailable", 0) / 1024 / 1024
            mem_cached_gb = mem.get("Cached", 0) / 1024 / 1024
            swap_total_gb = mem.get("SwapTotal", 0) / 1024 / 1024
            swap_free_gb = mem.get("SwapFree", 0) / 1024 / 1024
            swap_used_gb = max(0.0, swap_total_gb - swap_free_gb)
    except Exception:
        pass
    return mem_avail_gb, mem_cached_gb, swap_used_gb, swap_total_gb

def get_auxiliary_thermals():
    aux = {}
    p = find_hwmon_by_name("dell_ddv")
    if not p:
        p = find_hwmon_by_name("dell_smm")
        if p:
            smm_map = {
                "temp2_input": "VRM",
                "temp3_input": "Ambient",
                "temp5_input": "SODIMM",
                "temp6_input": "SODIMM 2",
                "temp7_input": "Charger",
            }
            for t, lbl in smm_map.items():
                t_path = os.path.join(p, t)
                if os.path.isfile(t_path):
                    try:
                        with open(t_path, "r") as f:
                            aux[lbl] = float(f.read().strip()) / 1000.0
                    except Exception:
                        pass
            return aux

    if p:
        for t in os.listdir(p):
            if t.startswith("temp") and t.endswith("_input"):
                lbl_file = os.path.join(p, t.replace("_input", "_label"))
                lbl = t
                if os.path.isfile(lbl_file):
                    try:
                        with open(lbl_file, "r") as f:
                            lbl = f.read().strip()
                    except Exception:
                        pass
                try:
                    with open(os.path.join(p, t), "r") as f:
                        val = float(f.read().strip()) / 1000.0
                        aux[lbl] = val
                        # Handle Dell hardware where VRM/Ambient are labeled as Unknown
                        if lbl == "Unknown":
                            if t == "temp2_input":
                                aux["VRM"] = val
                            elif t == "temp3_input":
                                aux["Ambient"] = val
                except Exception:
                    pass
    return aux

# Physical Hardware Profile Configurations
MODE_DEFS = {
    1: {"name": "battery", "profile": "quiet",       "boost": 10,  "label": "BATTERY SAVER",    "desc": "10% Boost (~1,000 RPM)"},
    2: {"name": "quiet",   "profile": "quiet",       "boost": 20,  "label": "QUIET",            "desc": "20% Boost (~1,450 RPM)"},
    3: {"name": "balance", "profile": "balanced",    "boost": 40,  "label": "BALANCE",          "desc": "40% Boost (~1,980 RPM)"},
    4: {"name": "mid",     "profile": "performance", "boost": 65,  "label": "MID",              "desc": "65% Boost (~3,100 RPM)"},
    5: {"name": "high",    "profile": "performance", "boost": 85,  "label": "HIGH PERFORMANCE", "desc": "85% Boost (~4,200 RPM)"},
    6: {"name": "ultra",   "profile": "performance", "boost": 255, "label": "ULTRA / G-MODE",   "desc": "100% Boost (5,029 RPM)"},
}

LAST_STATUS_MSG = None
LAST_STATUS_IS_ERROR = False
LAST_STATUS_TIMESTAMP = 0.0
NEEDS_RESIZE = False

def apply_mode_direct(mode_idx, aw_hwmon):
    global LAST_STATUS_MSG, LAST_STATUS_IS_ERROR, LAST_STATUS_TIMESTAMP
    cfg = MODE_DEFS.get(mode_idx)
    if not cfg:
        return False

    errs = []
    pprof_file = get_platform_profile_path()
    if pprof_file:
        ok, err = write_sysfs_value(pprof_file, cfg["profile"])
        if not ok:
            errs.append(f"platform_profile: {err}")
    else:
        errs.append("platform_profile not found")

    if aw_hwmon and os.path.isdir(aw_hwmon):
        for b_name in ["fan1_boost", "fan2_boost"]:
            b_path = os.path.join(aw_hwmon, b_name)
            if os.path.isfile(b_path):
                ok, err = write_sysfs_value(b_path, cfg["boost"])
                if not ok:
                    errs.append(f"{b_name}: {err}")
    else:
        errs.append("alienware_wmi hwmon not found")

    try:
        with open("/tmp/.awcc_gmode_state", "w") as f:
            f.write("ON\n" if cfg["boost"] == 255 else "OFF\n")
    except Exception:
        pass

    if errs:
        LAST_STATUS_MSG = f"Hardware Write Warning: {', '.join(errs)}"
        LAST_STATUS_IS_ERROR = True
        LAST_STATUS_TIMESTAMP = time.time()
        return False
    else:
        LAST_STATUS_MSG = f"Mode [{cfg['name'].upper()}] Applied: {cfg['label']} (Boost {cfg['boost']}/255)"
        LAST_STATUS_IS_ERROR = False
        LAST_STATUS_TIMESTAMP = time.time()

    try:
        is_turbo = (cfg["boost"] == 255)
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

    return True

def toggle_gameshift_direct(aw_hwmon):
    is_active = False
    try:
        if os.path.isfile("/tmp/.awcc_gmode_state"):
            with open("/tmp/.awcc_gmode_state", "r") as f:
                if f.read().strip() == "ON":
                    is_active = True
    except Exception:
        pass

    if not is_active and aw_hwmon and os.path.isdir(aw_hwmon):
        try:
            with open(os.path.join(aw_hwmon, "fan1_boost"), "r") as f:
                if int(f.read().strip()) == 255:
                    is_active = True
        except Exception:
            pass

    if is_active:
        apply_mode_direct(3, aw_hwmon)
    else:
        apply_mode_direct(6, aw_hwmon)

def decode_key(text):
    if not text:
        return None
    if any(k in text for k in ['\x1b[20~', '\x1b[19~', '\x1b[21~', '\x1b[24~', '\x1b[20', '\x1b[19', '\x1b[21']):
        return 'F9'
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
    if any(c in text for c in ['q', 'Q', 'x', 'X', '\x1b']) and len(text) == 1:
        return 'QUIT'
    if ('r' in text or 'R' in text) and not text.startswith('\x1b'):
        return 'REFRESH'
    if any(c in text for c in ['c', 'C', 't', 'T']) and not text.startswith('\x1b'):
        return 'THEME'
    return None

def build_cockpit_frame(cpu_usage, cpu_mhz, cpu_temp, gpu, b1, b2, r1, r2, profile,
                        mem_used, mem_total, ac_online, bat_pct, bat_status,
                        max1=5029, max2=5029, fans=None, rapl_watts=None, theme=None):
    if theme is None:
        theme = get_current_theme()

    box_width, inner_width, gauge_width, term_rows = get_box_dimensions()
    aw_hwmon = get_hwmon()
    vendor, product, display_machine = get_dmi_info()
    os_distro = get_os_release()
    tier_badge = get_verification_tier(vendor, aw_hwmon is not None)
    cpu_model = get_cpu_model()
    gpu_label = get_gpu_name()

    if fans is None:
        fans = [
            {"index": 1, "label": "FAN 1 (CPU)", "boost": b1, "rpm": r1, "max": max1, "controllable": aw_hwmon is not None},
            {"index": 2, "label": "FAN 2 (GPU)", "boost": b2, "rpm": r2, "max": max2, "controllable": aw_hwmon is not None},
        ]

    gmode_active = (b1 == 255)
    has_controllable_fans = any(f.get("controllable", False) for f in fans)

    if b1 >= 240:
        mode_label = "ULTRA (100% Boost)"
        mode_color = ansi_rgb_fg(255, 45, 75, bold=True)
    elif b1 >= 75:
        mode_label = "HIGH (85% Boost)"
        mode_color = ansi_rgb_fg(255, 120, 0, bold=True)
    elif b1 >= 50:
        mode_label = "MID (65% Boost)"
        mode_color = ansi_rgb_fg(255, 190, 0, bold=True)
    elif b1 >= 30:
        mode_label = "BALANCE (40% Boost)"
        mode_color = theme.accent_bright
    elif b1 >= 15:
        mode_label = "QUIET (20% Boost)"
        mode_color = ansi_rgb_fg(0, 255, 170, bold=True)
    elif b1 > 0:
        mode_label = "BATTERY (10% Boost)"
        mode_color = ansi_rgb_fg(180, 100, 255, bold=True)
    elif has_controllable_fans:
        mode_label = "PASSIVE (0% Boost)"
        mode_color = ansi_rgb_fg(180, 100, 255, bold=True)
    else:
        mode_label = f"PROFILE: {profile.upper()}"
        mode_color = theme.accent_bright

    if gmode_active:
        badge = f"{ansi_rgb_bg(255, 35, 65)}{ansi_rgb_fg(255, 255, 255, bold=True)}  G-MODE EXTREME  \033[0m"
    elif has_controllable_fans:
        badge = f"{theme.badge_bg}{theme.badge_fg}  ○ HYBRID QUIET  \033[0m"
    else:
        badge = f"{theme.badge_bg}{theme.badge_fg}  ● BIOS CONTROL  \033[0m"

    pwr_str = f"AC {bat_pct}% [ONLINE]" if ac_online else f"BAT {bat_pct}% [{bat_status.upper()}]"

    # Smooth thermal color ramp
    def temp_color(t):
        if t >= 85: return ansi_rgb_fg(255, 45, 75, bold=True)
        if t >= 72: return ansi_rgb_fg(255, 170, 0, bold=True)
        if t >= 58: return ansi_rgb_fg(0, 255, 170, bold=True)
        return ansi_rgb_fg(0, 220, 255)

    t_cpu_col = temp_color(cpu_temp)
    gauge_cpu = render_gauge(cpu_usage, 100, gauge_width, theme.stops_cpu, theme)

    g_temp = gpu["temp"]
    t_gpu_col = temp_color(g_temp)
    gauge_gpu = render_gauge(gpu["util"], 100, gauge_width, theme.stops_gpu, theme)

    vram_pct = (gpu["vram_used"] / gpu["vram_total"] * 100) if gpu["vram_total"] > 0 else 0.0
    vram_used_gb = gpu["vram_used"] / 1024.0
    vram_total_gb = gpu["vram_total"] / 1024.0
    gauge_vram = render_gauge(vram_pct, 100, gauge_width, theme.stops_vram, theme)

    ram_pct = (mem_used / mem_total * 100) if mem_total > 0 else 0.0
    gauge_ram = render_gauge(ram_pct, 100, gauge_width, theme.stops_ram, theme)

    top = f"{theme.border_corner}╔{theme.border}{'═' * (box_width - 2)}{theme.border_corner}╗{RESET}\033[K"
    div_double = f"{theme.border_corner}╠{theme.border}{'═' * (box_width - 2)}{theme.border_corner}╣{RESET}\033[K"
    div_single = f"{theme.border_corner}╟{theme.border}{'─' * (box_width - 2)}{theme.border_corner}╢{RESET}\033[K"
    bot = f"{theme.border_corner}╚{theme.border}{'═' * (box_width - 2)}{theme.border_corner}╝{RESET}\033[K"

    rows = []
    rows.append(top)

    # 1. Header Block
    hdr_left = f"{theme.title}▲ ALIENWARE COMMAND CENTER v2.5{RESET}"
    hdr_center = f"{WHITE_BOLD}{display_machine.upper()}{RESET}"
    hdr_right = f"{MUTED_GRAY}{os_distro} {tier_badge}{RESET}"
    rows.append(make_row(align_three(hdr_left, hdr_center, hdr_right, inner_width), inner_width, theme))
    rows.append(div_double)

    # 2. Status Block
    stat_left = f"{MUTED_GRAY}MODE:{RESET} {mode_color}{mode_label}{RESET}"
    stat_right = f"{MUTED_GRAY}PWR:{RESET} {WHITE_BOLD}{pwr_str}{RESET}"
    rows.append(make_row(align_three(stat_left, badge, stat_right, inner_width), inner_width, theme))
    rows.append(div_double)

    # Status / Error Banner
    now = time.time()
    if LAST_STATUS_MSG and (now - LAST_STATUS_TIMESTAMP < 4.0):
        if LAST_STATUS_IS_ERROR:
            msg = f"{ansi_rgb_bg(255, 45, 75)}{ansi_rgb_fg(255, 255, 255, bold=True)}  [!] {LAST_STATUS_MSG}  \033[0m"
        else:
            msg = f"{ansi_rgb_bg(0, 140, 100)}{ansi_rgb_fg(255, 255, 255, bold=True)}  ✓ {LAST_STATUS_MSG}  \033[0m"
        rows.append(make_row(align_three("", msg, "", inner_width), inner_width, theme))
        rows.append(div_single)

    # 3. Cooling Turbines Block
    def boost_pct_str(b):
        if b >= 240: return "100%"
        if b >= 75:  return " 85%"
        if b >= 50:  return " 65%"
        if b >= 30:  return " 40%"
        if b >= 15:  return " 20%"
        if b > 0:    return " 10%"
        return "  0%"

    max_ceiling = max([f["max"] for f in fans]) if fans else 5029
    cool_left = f"{theme.title}[ DUAL HIGH-PRESSURE COOLING TURBINES ]{RESET}"
    cool_right = f"{MUTED_GRAY}MAX CEILING: {max_ceiling:,d} RPM{RESET}"
    rows.append(make_row(align_split(cool_left, cool_right, inner_width), inner_width, theme))

    for f_info in fans:
        f_gauge = render_gauge(f_info["rpm"], f_info["max"], gauge_width, theme.stops_cool, theme)
        f_val = f"{WHITE_BOLD}{f_info['rpm']:4d}RPM{RESET}"
        if f_info.get("controllable", False):
            b = f_info["boost"]
            f_mid = f"Boost: {theme.accent_bright}{b:3d} ({boost_pct_str(b)}){RESET}"
        else:
            f_mid = f"Control: {MUTED_GRAY}FIRMWARE AUTO{RESET}"
        f_right = f"{MUTED_GRAY}Max: {f_info['max']:,d} RPM{RESET}"
        rows.append(format_telemetry_line(f"{WHITE_BOLD}{f_info['label']}{RESET}", f_gauge, f_val, f_mid, f_right, inner_width, gauge_width))

    # Prominently show Motherboard VRM, Ambient, SODIMM & Charger readings
    aux_temps = get_auxiliary_thermals()
    vrm_val = aux_temps.get("VRM")
    amb_val = aux_temps.get("Ambient")
    sod_val = aux_temps.get("SODIMM")
    sod2_val = aux_temps.get("SODIMM 2")
    chg_val = aux_temps.get("Charger")

    parts = []
    if vrm_val: parts.append(f"VRM: {temp_color(vrm_val)}{vrm_val:.1f}°C{RESET}")
    if amb_val: parts.append(f"Air: {temp_color(amb_val)}{amb_val:.1f}°C{RESET}")
    if sod_val:
        if sod2_val:
            parts.append(f"RAM: {temp_color(sod_val)}{sod_val:.0f}°C/{sod2_val:.0f}°C{RESET}")
        else:
            parts.append(f"RAM: {temp_color(sod_val)}{sod_val:.1f}°C{RESET}")
    if chg_val: parts.append(f"Charger: {temp_color(chg_val)}{chg_val:.1f}°C{RESET}")
    if parts:
        matrix_line = f" {MUTED_GRAY}•{RESET} ".join(parts)
        rows.append(make_row(align_split(f"{theme.accent}├─ SENSORS{RESET}", matrix_line, inner_width), inner_width, theme))

    rows.append(div_double)

    # 4. Silicon Computation & Thermals Block
    silicon_left = f"{theme.title}[ SILICON COMPUTATION & THERMALS ]{RESET}"
    silicon_right = f"{MUTED_GRAY}ACPI Platform Profile Active{RESET}"
    rows.append(make_row(align_split(silicon_left, silicon_right, inner_width), inner_width, theme))

    cpu_val = f"{theme.accent_bright}{cpu_usage:5.1f}%{RESET}"
    cpu_pwr_str = f" | {rapl_watts:.0f}W" if rapl_watts else ""
    cpu_mid = f"Clock: {WHITE_BOLD}{cpu_mhz:4.0f} MHz{RESET}{cpu_pwr_str}"
    cpu_right = f"{t_cpu_col}Temp: {cpu_temp:4.1f}°C{RESET}"
    rows.append(format_telemetry_line(f"{WHITE_BOLD}{cpu_model} CPU{RESET}", gauge_cpu, cpu_val, cpu_mid, cpu_right, inner_width, gauge_width))

    gpu_val = f"{theme.title}{gpu['util']:5.1f}%{RESET}"
    gpu_pwr_str = f" | {gpu['pwr_str']}" if gpu.get("pwr_str") else ""
    gpu_mid = f"Clock: {WHITE_BOLD}{gpu['clk']:4d} MHz{RESET}{gpu_pwr_str}"
    gpu_right = f"{t_gpu_col}Temp: {g_temp:4.1f}°C{RESET}"
    rows.append(format_telemetry_line(f"{WHITE_BOLD}{gpu_label}{RESET}", gauge_gpu, gpu_val, gpu_mid, gpu_right, inner_width, gauge_width))

    vram_val = f"{f_info_col if 'f_info_col' in locals() else theme.accent}{vram_pct:5.1f}%{RESET}"
    vram_mid = f"Used:  {WHITE_BOLD}{vram_used_gb:4.2f} GB{RESET}"
    vram_right = f"{MUTED_GRAY}Limit: {vram_total_gb:4.2f} GB{RESET}"
    rows.append(format_telemetry_line(f"{WHITE_BOLD}GDDR6 VRAM{RESET}", gauge_vram, vram_val, vram_mid, vram_right, inner_width, gauge_width))

    ram_val = f"{theme.accent_bright}{ram_pct:5.1f}%{RESET}"
    if sod_val:
        if sod2_val:
            ram_mid = f"Used:  {WHITE_BOLD}{mem_used:4.1f} GB{RESET} • {temp_color(sod_val)}{sod_val:.0f}°C{RESET}"
        else:
            ram_mid = f"Used:  {WHITE_BOLD}{mem_used:4.1f} GB{RESET} • {temp_color(sod_val)}{sod_val:.0f}°C{RESET}"
    else:
        ram_mid = f"Used:  {WHITE_BOLD}{mem_used:4.1f} GB{RESET}"
    ram_right = f"{MUTED_GRAY}Total: {mem_total:4.1f} GB{RESET}"
    rows.append(format_telemetry_line(f"{WHITE_BOLD}SYSTEM RAM{RESET}", gauge_ram, ram_val, ram_mid, ram_right, inner_width, gauge_width))

    # Footer Block
    footer_rows = []
    footer_rows.append(div_double)
    th_label = theme.name.upper() if inner_width >= 88 else theme.key.upper()
    if has_controllable_fans:
        hk_left = f"{theme.accent}HARDWARE HOTKEY:{RESET} {WHITE_BOLD}F9 / Fn+F9{RESET} (Game Shift Toggle)"
        hk_right = f"{MUTED_GRAY}THEME: {th_label}{RESET}"
    else:
        hk_left = f"{theme.accent}PROFILE HOTKEY:{RESET} {WHITE_BOLD}1-6{RESET} (Platform Thermal Profiles)"
        hk_right = f"{MUTED_GRAY}THEME: {th_label}{RESET}"
    footer_rows.append(make_row(align_split(hk_left, hk_right, inner_width), inner_width, theme))
    footer_rows.append(div_single)

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
        (1, "Battery", "Bat", "\033[1;97;45m"),
        (2, "Quiet",   "Qut", "\033[1;30;42m"),
        (3, "Balance", "Bal", "\033[1;30;46m"),
        (4, "Mid",     "Mid", "\033[1;30;43m"),
        (5, "High",    "High", "\033[1;97;41m"),
        (6, "Ultra",   "Ult", "\033[1;97;41m"),
    ]
    mode_spans = []
    use_short = (inner_width < 70)
    for idx, name, sname, bg_col in mode_defs_list:
        display_name = sname if use_short else name
        if idx == active_idx:
            mode_spans.append(f"{bg_col} [{idx}] {display_name} \033[0m")
        else:
            mode_spans.append(f"{MUTED_GRAY}[{idx}] {display_name}{RESET}")
    sep = " " if use_short else "  "
    modes_line = f"{WHITE_BOLD}MODES:{RESET} " + sep.join(mode_spans)
    footer_rows.append(make_row(modes_line, inner_width, theme))

    if inner_width >= 72:
        acts_line = f"{theme.accent_bright}ACTIONS:{RESET} [F9] GameShift  •  [1-6] Profiles  •  [C/T] Theme  •  [Q] Exit"
    else:
        acts_line = f"{theme.accent_bright}ACTIONS:{RESET} [F9] Turbo  •  [1-6] Modes  •  [C] Theme  •  [Q] Exit"
    footer_rows.append(make_row(acts_line, inner_width, theme))
    footer_rows.append(bot)

    rem = term_rows - (len(rows) + len(footer_rows))

    # Dynamic Panel 1: System Governor, Load Averages & Uptime
    if rem >= 3:
        rows.append(div_double)
        load1, load5, load15, uptime_str, gov_str = get_system_diagnostics()
        diag_left = f"{theme.title}[ SYSTEM ENVIRONMENT & GOVERNOR ]{RESET}"
        diag_right = f"{MUTED_GRAY}Kernel 6.12 • ACPI WMAX Active{RESET}"
        rows.append(make_row(align_split(diag_left, diag_right, inner_width), inner_width, theme))

        load_t = f"Load Avg: {WHITE_BOLD}{load1:.2f}, {load5:.2f}, {load15:.2f}{RESET}"
        gov_t = f"Governor: {theme.accent_bright}{gov_str.upper()}{RESET}"
        up_t = f"Uptime: {WHITE_BOLD}{uptime_str}{RESET}"
        rows.append(make_row(align_three(load_t, gov_t, up_t, inner_width), inner_width, theme))
        rem -= 3

    # Dynamic Panel 2: Auxiliary Thermals (SODIMM, Charger)
    if rem >= 3 and aux_temps:
        rows.append(div_double)
        aux_left = f"{theme.accent}[ AUXILIARY THERMAL SENSORS ]{RESET}"
        aux_right = f"{MUTED_GRAY}Subsystem: dell_ddv{RESET}"
        rows.append(make_row(align_split(aux_left, aux_right, inner_width), inner_width, theme))

        sod1 = f"SODIMM 1: {WHITE_BOLD}{aux_temps.get('SODIMM', 39):.0f}°C{RESET}"
        sod2 = f"SODIMM 2: {WHITE_BOLD}{aux_temps.get('SODIMM 2', 39):.0f}°C{RESET}"
        chg = f"Charger: {WHITE_BOLD}{aux_temps.get('Charger', 37):.0f}°C{RESET}"
        rows.append(make_row(align_three(sod1, sod2, chg, inner_width), inner_width, theme))
        rem -= 3

    # Dynamic Panel 3: Memory Subsystem & Swap
    if rem >= 3:
        mem_avail, mem_cached, swap_used, swap_total = get_memory_breakdown()
        rows.append(div_double)
        mem_left = f"{theme.accent}[ MEMORY SUBSYSTEM & SWAP ]{RESET}"
        mem_right = f"{MUTED_GRAY}Available: {mem_avail:.1f} GB{RESET}"
        rows.append(make_row(align_split(mem_left, mem_right, inner_width), inner_width, theme))

        swap_pct = (swap_used / swap_total * 100) if swap_total > 0 else 0.0
        gauge_swap = render_gauge(swap_pct, 100, gauge_width, theme.stops_cool, theme)
        swap_val = f"{theme.accent_bright}{swap_pct:5.1f}%{RESET}"
        swap_mid = f"Used:  {WHITE_BOLD}{swap_used:4.1f} GB{RESET}"
        swap_right = f"{MUTED_GRAY}Total: {swap_total:4.1f} GB{RESET}"
        rows.append(format_telemetry_line(f"{WHITE_BOLD}SWAP FILE{RESET}", gauge_swap, swap_val, swap_mid, swap_right, inner_width, gauge_width))
        rem -= 3

    if rem > 0:
        div_indices = [i for i, r in enumerate(rows) if "╠" in r and i > 2]
        if div_indices:
            for i in range(rem):
                target_div = div_indices[i % len(div_indices)]
                rows.insert(target_div, make_row("", inner_width, theme))
                div_indices = [d + 1 if d >= target_div else d for d in div_indices]
        else:
            while rem > 0:
                rows.append(make_row("", inner_width, theme))
                rem -= 1

    if rem < 0:
        if len(footer_rows) > 5 and rem <= -1:
            footer_rows.pop(1)
            footer_rows.pop(1)
            rem += 2
        while rem < 0 and len(rows) > 12:
            rows.pop()
            rem += 1

    rows.extend(footer_rows)
    return "\n".join(rows)

def run_snapshot(theme=None):
    if theme is None:
        theme = get_current_theme()
    aw_hwmon = get_hwmon()
    max1, max2 = get_fan_max(aw_hwmon)
    _, _, cpu_usage, cpu_mhz, cpu_temp, rapl_watts, _, _ = get_cpu_info(0, 0)
    gpu = get_gpu_info()
    b1, b2, r1, r2, fans = get_fans(aw_hwmon)

    profile = "balanced"
    pprof_file = get_platform_profile_path()
    if pprof_file and os.path.isfile(pprof_file):
        try:
            with open(pprof_file, "r") as f:
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
    frame = build_cockpit_frame(cpu_usage, cpu_mhz, cpu_temp, gpu, b1, b2, r1, r2, profile,
                                mem_used, mem_total, ac_online, bat_pct, bat_status,
                                max1, max2, fans=fans, rapl_watts=rapl_watts, theme=theme)
    print(frame)

def run_continuous():
    global NEEDS_RESIZE, LAST_STATUS_MSG, LAST_STATUS_IS_ERROR, LAST_STATUS_TIMESTAMP
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
        sys.stdout.write("\033[?1049l\033[?25h" + RESET + "\n")
        sys.stdout.flush()

    def sig_handler(sig, frame):
        cleanup()
        sys.exit(0)

    def on_winch(sig, frame):
        global NEEDS_RESIZE
        NEEDS_RESIZE = True

    signal.signal(signal.SIGINT, sig_handler)
    signal.signal(signal.SIGTERM, sig_handler)
    signal.signal(signal.SIGWINCH, on_winch)

    sys.stdout.write("\033[?1049h\033[?1007l\033[?25l\033[2J\033[H")
    sys.stdout.flush()

    prev_total, prev_idle = 0.0, 0.0
    prev_rapl_uj, prev_rapl_time = 0, 0.0
    aw_hwmon = get_hwmon()
    needs_quick_redraw = False

    try:
        while True:
            if NEEDS_RESIZE:
                NEEDS_RESIZE = False
                sys.stdout.write("\033[2J\033[H")
                sys.stdout.flush()

            timeout = 0.05 if needs_quick_redraw else 0.75
            needs_quick_redraw = False

            if fd is not None:
                try:
                    r, _, _ = select.select([sys.stdin], [], [], timeout)
                except (InterruptedError, select.error):
                    r = []

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
                    elif action == 'THEME':
                        t = cycle_next_theme()
                        LAST_STATUS_MSG = f"Color Grading Theme: {t.name}"
                        LAST_STATUS_IS_ERROR = False
                        LAST_STATUS_TIMESTAMP = time.time()
                        needs_quick_redraw = True
                    elif action == 'REFRESH':
                        needs_quick_redraw = True
            else:
                time.sleep(0.75)

            max1, max2 = get_fan_max(aw_hwmon)
            prev_total, prev_idle, cpu_usage, cpu_mhz, cpu_temp, rapl_watts, prev_rapl_uj, prev_rapl_time = get_cpu_info(
                prev_total, prev_idle, prev_rapl_uj, prev_rapl_time
            )
            gpu = get_gpu_info()
            b1, b2, r1, r2, fans = get_fans(aw_hwmon)

            profile = "balanced"
            pprof_file = get_platform_profile_path()
            if pprof_file and os.path.isfile(pprof_file):
                try:
                    with open(pprof_file, "r") as f:
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

            frame = build_cockpit_frame(cpu_usage, cpu_mhz, cpu_temp, gpu, b1, b2, r1, r2, profile,
                                        mem_used, mem_total, ac_online, bat_pct, bat_status,
                                        max1, max2, fans=fans, rapl_watts=rapl_watts)

            sys.stdout.write("\033[H" + frame)
            sys.stdout.flush()

    finally:
        cleanup()

if __name__ == "__main__":
    if "--help" in sys.argv or "-h" in sys.argv:
        print("""AWCC Cybernetic Lux Dashboard (TrueColor Hardware Cockpit)
Usage: awcc-lux-dashboard.py [OPTIONS]

Options:
  -s, --snapshot         Print single-frame status snapshot and exit
  --theme <name>         Theme: cryo, cyberpunk, darkcore, solar, matrix, vaporwave
  --sysfs-root <path>    Override sysfs root directory for testing
  -h, --help             Show this help message and exit

Interactive Keybindings:
  [1-6]                  Switch Performance Profiles (1: Battery ... 6: Ultra)
  [F9]                   Hardware Game Shift Boost (Toggle G-Mode)
  [C] / [T]              Cycle TrueColor Color Grading Themes
  [Q]                    Quit dashboard""")
        sys.exit(0)

    if "--sysfs-root" in sys.argv:
        idx = sys.argv.index("--sysfs-root")
        if idx + 1 < len(sys.argv):
            SYSFS_ROOT = sys.argv[idx + 1]

    if "--theme" in sys.argv:
        idx = sys.argv.index("--theme")
        if idx + 1 < len(sys.argv):
            req_theme = sys.argv[idx + 1].lower()
            if req_theme in THEMES:
                CURRENT_THEME_KEY = req_theme
                save_persisted_theme(req_theme)

    if "--snapshot" in sys.argv or "-s" in sys.argv:
        run_snapshot()
    else:
        run_continuous()
