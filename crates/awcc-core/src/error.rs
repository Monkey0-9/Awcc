use thiserror::Error;

#[derive(Error, Debug)]
pub enum AwccError {
    #[error("AWCC driver (alienware_wmi) hwmon node not found in sysfs")]
    DriverNotFound,

    #[error("Dell SMM hwmon device not found")]
    DellSmmNotFound,

    #[error("Platform profile device not found")]
    PlatformProfileNotFound,

    #[error("I/O error accessing sysfs path '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Permission denied accessing '{path}'. Root or sudo privilege required.")]
    PermissionDenied { path: String },

    #[error("Failed to parse integer from '{path}': '{content}'")]
    ParseInt { path: String, content: String },

    #[error("Boost verification failed for fan {fan}: requested {requested}, readback {readback}")]
    ReadbackMismatch {
        fan: u8,
        requested: u8,
        readback: u8,
    },

    #[error("Invalid fan index {0}; expected controllable fan index")]
    InvalidFanIndex(u8),

    #[error("Invalid platform profile '{requested}'. Available choices: [{}]", available.join(", "))]
    InvalidProfile {
        requested: String,
        available: Vec<String>,
    },

    #[error("Profile verification failed: requested '{requested}', readback '{readback}'")]
    ProfileReadbackMismatch { requested: String, readback: String },

    #[error("Platform profile error: {0}")]
    PlatformProfile(String),

    #[error("Sensor unavailable: {0}")]
    SensorUnavailable(String),
}
