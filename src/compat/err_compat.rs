//! Error compatibility layer for embedded-hal 0.2 and 1.0
//! Provides a unified error type for logging and diagnostics in no_std
//! HAL error compatibility wrapper
//! Converts HAL-specific errors into dvcdbg's canonical ErrorKind

use core::fmt::Debug;
use crate::error::*;

#[cfg(feature = "ehal_0_2")]
use embedded_hal_0_2::blocking::i2c;
#[cfg(feature = "ehal_1_0")]
use embedded_hal_1::i2c;

/// Trait for HAL error extensions
pub trait HalErrorExt {
    /// Check if the error corresponds to a non-blocking/would-block situation
    fn is_would_block(&self) -> bool;

    /// Convert the HAL error into canonical ErrorKind
    fn to_compat(&self, addr: Option<u8>) -> ErrorKind;
}

#[cfg(feature = "ehal_0_2")]
impl<E> HalErrorExt for E
where
    E: Debug,
{
    fn is_would_block(&self) -> bool {
        // For ehal 0.2, we heuristically detect NACK via Debug output
        let mut buf = heapless::String::<64>::new();
        let _ = core::fmt::write(&mut buf, format_args!("{:?}", self));
        buf.contains("NACK") || buf.contains("NoAcknowledge")
    }

    fn to_compat(&self, addr: Option<u8>) -> ErrorKind {
        // Map to canonical ErrorKind
        if self.is_would_block() {
            if let Some(a) = addr {
                ErrorKind::I2c(I2cError::Nack)
            } else {
                ErrorKind::I2c(I2cError::Nack)
            }
        } else {
            ErrorKind::Unknown
        }
    }
}

#[cfg(feature = "ehal_1_0")]
impl<E> HalErrorExt for E
where
    E: i2c::Error + Debug,
{
    fn is_would_block(&self) -> bool {
        matches!(self.kind(), i2c::ErrorKind::NoAcknowledge(_))
    }

    fn to_compat(&self, addr: Option<u8>) -> ErrorKind {
        let kind = match self.kind() {
            i2c::ErrorKind::Bus => ErrorKind::I2c(I2cError::Bus),
            i2c::ErrorKind::NoAcknowledge(_) => ErrorKind::I2c(I2cError::Nack),
            i2c::ErrorKind::ArbitrationLoss => ErrorKind::I2c(I2cError::ArbitrationLost),
            _ => ErrorKind::Unknown,
        };
        if let Some(_a) = addr {
            kind // could extend to include addr if needed
        } else {
            kind
        }
    }
}
