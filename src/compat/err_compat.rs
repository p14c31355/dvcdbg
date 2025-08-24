//! src/compat/err_compat.rs
//! Error compatibility layer for embedded-hal 0.2 and 1.0
//! Provides a unified error type for logging and diagnostics in no_std
//! HAL error compatibility wrapper
//! Works with embedded-hal 0.2 and 1.0, maps to unified `ErrorKind`

use crate::error::*;
use core::fmt::Debug;
#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
use core::fmt::Write;

#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
use heapless::String;

#[cfg(feature = "ehal_1_0")]
use embedded_hal_1::i2c as i2c_1_0;

/// Trait for HAL error extensions
pub trait HalErrorExt {
    /// Convert HAL error into unified `ErrorKind`, optionally with device address
    fn to_compat(&self, addr: Option<u8>) -> ErrorKind;
}

#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
impl<E> HalErrorExt for E
where
    E: Debug,
{
    fn to_compat(&self, _addr: Option<u8>) -> ErrorKind {
        // Map 0.2 HAL error to unified ErrorKind
        // NOTE: 0.2 uses Debug output to detect NACKs
        let mut buf: String<128> = String::new();
        let _ = write!(buf, "{:?}", self);
        if buf.contains("NACK") || buf.contains("NoAcknowledge") {
            ErrorKind::I2c(I2cError::Nack)
        } else {
            ErrorKind::Unknown
        }
    }
}

#[cfg(feature = "ehal_1_0")]
impl<E> HalErrorExt for E
where
    E: i2c_1_0::Error + Debug,
{
    fn to_compat(&self, _addr: Option<u8>) -> ErrorKind {
        // Convert 1.0 HAL error into unified ErrorKind
        match self.kind() {
            i2c_1_0::ErrorKind::Bus => ErrorKind::I2c(I2cError::Bus),
            i2c_1_0::ErrorKind::NoAcknowledge(_) => ErrorKind::I2c(I2cError::Nack),
            i2c_1_0::ErrorKind::ArbitrationLoss => ErrorKind::I2c(I2cError::ArbitrationLost),
            _ => ErrorKind::Unknown,
        }
    }
}
