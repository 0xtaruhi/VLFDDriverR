use std::{error::Error as StdError, fmt::Display};

#[derive(Debug)]
pub enum DeviceError {
    OpenError,
    ReadError(String),
    WriteError(String),
    CloseError(String),
    OtherError(String),
}

impl Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            DeviceError::OpenError => write!(f, "Device Open error"),
            DeviceError::ReadError(ref s) => write!(f, "Device read error: {}", s),
            DeviceError::WriteError(ref s) => write!(f, "Device write error: {}", s),
            DeviceError::CloseError(ref s) => write!(f, "Device close error {}", s),
            DeviceError::OtherError(ref s) => write!(f, "Device other error: {}", s),
        }
    }
}

impl StdError for DeviceError {
    fn description(&self) -> &str {
        match *self {
            DeviceError::OpenError => "Device Open error",
            DeviceError::ReadError(_) => "Device read error",
            DeviceError::WriteError(_) => "Device write error",
            DeviceError::CloseError(_) => "Device close error",
            DeviceError::OtherError(_) => "Device other error",
        }
    }

    fn cause(&self) -> Option<&dyn StdError> {
        None
    }
}
