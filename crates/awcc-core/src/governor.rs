//! Autonomous thermal governor logic with anti-hunt hysteresis.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn last_boost(&self) -> u8 {
        self.last_boost
    }

    /// Evaluates piecewise-linear boost curve with anti-hunt hysteresis.
    ///
    /// - Below 50.0°C: boost is 0 (fan boost disengaged).
    /// - 85.0°C and above: boost is 255 (maximum cooling).
    /// - 50.0°C to 85.0°C: linear interpolation between 0 and 255.
    /// - Hysteresis: on temperature drops, damping restricts decrease to at most 15 units per cycle.
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
