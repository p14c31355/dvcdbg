//! Error compatibility layer for embedded-hal 0.2 and 1.0
//! Provides a unified error type for logging and diagnostics in no_std

use core::fmt::{Debug, Write}; // import Write for heapless::String
use crate::error::ErrorKind;
use heapless::String;

#[cfg(feature = "ehal_1_0")]
use embedded_hal_1::i2c;

/// Unified error enum for embedded diagnostics
#[derive(Debug, Clone, Copy)]
pub enum ErrorCompat {
    /// I2C bus error (addr, kind)
    I2cError(u8, ErrorKind),
    /// Generic HAL error
    HalError(ErrorKind),
}

impl ErrorCompat {
    pub fn kind(&self) -> ErrorKind {
        match self {
            ErrorCompat::I2cError(_, kind) => *kind,
            ErrorCompat::HalError(kind) => *kind,
        }
    }
}

/// Trait for HAL-specific error extensions
pub trait HalErrorExt {
    fn is_would_block(&self) -> bool;
    fn to_compat(&self, addr: Option<u8>) -> ErrorCompat;
}

#[cfg(feature = "ehal_0_2")]
impl<E> From<E> for ErrorCompat
where
    E: Debug,
{
    fn from(e: E) -> Self {
        // NOTE: Fallback Debug-based detection for hal-0.2 errors
        let mut buf: String<64> = String::new();
        let _ = write!(buf, "{:?}", e);
        if buf.contains("NACK") || buf.contains("NoAcknowledge") {
            ErrorCompat::I2cError(0, ErrorKind::I2cNack)
        } else {
            ErrorCompat::HalError(ErrorKind::Unknown)
        }
    }
}

#[cfg(feature = "ehal_0_2")]
impl<E> HalErrorExt for E
where
    E: Debug,
{
    fn is_would_block(&self) -> bool {
        // NOTE: For embedded-hal 0.2, detect NACKs via Debug output
        let mut buf: String<64> = String::new();
        let _ = write!(buf, "{:?}", self);
        buf.contains("NACK") || buf.contains("NoAcknowledge")
    }

    fn to_compat(&self, addr: Option<u8>) -> ErrorCompat {
        // Convert HAL error to unified enum
        if let Some(a) = addr {
            ErrorCompat::I2cError(a, ErrorKind::Unknown)
        } else {
            ErrorCompat::HalError(ErrorKind::Unknown)
        }
    }
}

#[cfg(feature = "ehal_1_0")]
impl<E> From<E> for ErrorCompat
where
    E: i2c::Error + Debug,
{
    fn from(e: E) -> Self {
        let kind = match e.kind() {
            i2c::ErrorKind::Bus => ErrorKind::I2cBus,
            i2c::ErrorKind::NoAcknowledge(_) => ErrorKind::I2cNack,
            i2c::ErrorKind::ArbitrationLoss => ErrorKind::I2cArbitrationLost,
            _ => ErrorKind::Unknown,
        };
        ErrorCompat::HalError(kind)
    }
}

#[cfg(feature = "ehal_1_0")]
impl<E> HalErrorExt for E
where
    E: i2c::Error + Debug,
{
    fn is_would_block(&self) -> bool {
        // NOTE: Use embedded-hal 1.0 standardized ErrorKind
        matches!(self.kind(), i2c::ErrorKind::NoAcknowledge(_))
    }

    fn to_compat(&self, addr: Option<u8>) -> ErrorCompat {
        // Convert HAL error to unified enum
        let kind = match self.kind() {
            i2c::ErrorKind::Bus => ErrorKind::I2cBus,
            i2c::ErrorKind::NoAcknowledge(_) => ErrorKind::I2cNack,
            i2c::ErrorKind::ArbitrationLoss => ErrorKind::I2cArbitrationLost,
            _ => ErrorKind::Unknown,
        };
        if let Some(a) = addr {
            ErrorCompat::I2cError(a, kind)
        } else {
            ErrorCompat::HalError(kind)
        }
    }
}
