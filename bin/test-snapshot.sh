#!/bin/bash
set -euo pipefail

# CPU
line1=$(grep '^cpu ' /proc/stat)
sleep 0.2
line2=$(grep '^cpu ' /proc/stat)
u1=$(echo "$line1" | awk '{print $2+$3+$4+$5+$6+$7+$8}')
i1=$(echo "$line1" | awk '{print $5}')
u2=$(echo "$line2" | awk '{print $2+$3+$4+$5+$6+$7+$8}')
i2=$(echo "$line2" | awk '{print $5}')
diff_total=$(( u2 - u1 ))
diff_idle=$(( i2 - i1 ))
cpu_util=$(( 100 * (diff_total - diff_idle) / diff_total ))
cpu_mhz=$(awk '/cpu MHz/{s+=$4; n++} END{if(n>0) printf "%.0f", s/n}' /proc/cpuinfo)
cpu_temp=$(cat /sys/class/hwmon/hwmon5/temp1_input 2>/dev/null | awk '{print $1/1000}')

# GPU
gpu_info=$(nvidia-smi --query-gpu=utilization.gpu,temperature.gpu,clocks.current.graphics,power.draw --format=csv,noheader,nounits 2>/dev/null || echo "0, 0, 0, 0")

# Fans
r1=$(cat /sys/class/hwmon/hwmon7/fan1_input 2>/dev/null || cat /sys/class/hwmon/hwmon4/fan1_input 2>/dev/null || echo "0")
r2=$(cat /sys/class/hwmon/hwmon7/fan2_input 2>/dev/null || cat /sys/class/hwmon/hwmon4/fan2_input 2>/dev/null || echo "0")
b1=$(cat /sys/class/hwmon/hwmon7/fan1_boost 2>/dev/null || echo "0")
b2=$(cat /sys/class/hwmon/hwmon7/fan2_boost 2>/dev/null || echo "0")

echo "=========================================================================="
echo "          AWCC LIVE SYSTEM TRACKING TELEMETRY SNAPSHOT                   "
echo "=========================================================================="
echo " [CPU] Load: ${cpu_util}% | Clock: ${cpu_mhz} MHz | Temperature: ${cpu_temp}°C"
echo " [GPU] Load: $(echo "$gpu_info" | cut -d',' -f1)% | Core Clock: $(echo "$gpu_info" | cut -d',' -f3) MHz | Temp: $(echo "$gpu_info" | cut -d',' -f2)°C | Power: $(echo "$gpu_info" | cut -d',' -f4) W"
echo " [FANS] Fan 1 (CPU): ${r1} RPM (boost: ${b1}/255) | Fan 2 (GPU): ${r2} RPM (boost: ${b2}/255)"
echo " [PROFILE] $(cat /sys/firmware/acpi/platform_profile 2>/dev/null)"
echo "=========================================================================="
