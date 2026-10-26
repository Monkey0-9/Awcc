use thiserror::Error;

#[derive(Error, Debug)]
pub enum AwccError {
    #[error("AWCC driver (alienware_wmi) hwmon node not found in sysfs")]
    DriverNotFound,

    #[error("Dell SMM hwmon device not found for tachometer cross-verification")]
    DellSmmNotFound,

    #[error("I/O error accessing sysfs path '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse integer from '{path}': {content}")]
    ParseInt {
        path: String,
        content: String,
    },

    #[error("Boost verification failed for fan {fan}: requested {requested}, readback {readback}")]
    ReadbackMismatch {
        fan: u8,
        requested: u8,
        readback: u8,
    },

    #[error("Invalid fan index {0}; expected 1 or 2")]
    InvalidFanIndex(u8),

    #[error("Platform profile error: {0}")]
    PlatformProfile(String),
}
