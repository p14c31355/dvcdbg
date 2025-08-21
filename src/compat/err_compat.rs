//! Error compatibility layer for embedded-hal 0.2 and 1.0
//! Provides a unified error type for logging and diagnostics in no_std


use core::fmt::Debug;

#[cfg(feature = "ehal_0_2")]
use embedded_hal_0_2::i2c;
#[cfg(feature = "ehal_1_0")]
use embedded_hal_1::i2c;

/// Unified error enum for embedded diagnostics
#[derive(Debug, Clone, Copy)]
pub enum ErrorCompat {
    /// I2C bus error
    I2cError(u8), // Placeholder: address where error occurred
    /// Generic HAL error
    HalError,
}

/// Trait for HAL-specific error extensions
pub trait HalErrorExt {
    fn is_would_block(&self) -> bool;
    fn to_compat(&self, addr: Option<u8>) -> ErrorCompat;
}

#[cfg(feature = "ehal_0_2")]
impl<E> HalErrorExt for E
where
    E: Debug,
{
    fn is_would_block(&self) -> bool {
        // NOTE: For embedded-hal 0.2, detect NACKs via Debug output
        let mut buf: String<U64> = String::new();
        let _ = write!(buf, "{:?}", self);
        buf.contains("NACK") || buf.contains("NoAcknowledge")
    }

    fn to_compat(&self, addr: Option<u8>) -> ErrorCompat {
        // Convert HAL error to unified enum
        if let Some(a) = addr {
            ErrorCompat::I2cError(a)
        } else {
            ErrorCompat::HalError
        }
    }
}

#[cfg(feature = "ehal_1_0")]
impl<E> HalErrorExt for E
where
    E: i2c::Error + Debug,
{
    fn is_would_block(&self) -> bool {
        // NOTE: Use embedded-hal 1.0 standardized ErrorKind
        matches!(self.kind(), i2c::ErrorKind::WouldBlock)
    }

    fn to_compat(&self, addr: Option<u8>) -> ErrorCompat {
        // Convert HAL error to unified enum
        if let Some(a) = addr {
            ErrorCompat::I2cError(a)
        } else {
            ErrorCompat::HalError
        }
    }
}
