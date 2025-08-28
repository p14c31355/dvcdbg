//! src/compat/err_compat.rs
//! HAL error compatibility layer for embedded-hal 0.2 and 1.0
//! Provides a unified `ErrorKind` for diagnostics.

use crate::error::*;
use core::fmt::Debug;

#[cfg(feature = "ehal_1_0")]
use embedded_hal_1::i2c as i2c_1_0;

/// Trait to convert HAL errors into unified `ErrorKind`
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
        ErrorKind::I2c(I2cError::Nack)
    }
}

#[cfg(feature = "ehal_1_0")]
impl<E> HalErrorExt for E
where
    E: i2c_1_0::Error + Debug,
{
    fn to_compat(&self, _addr: Option<u8>) -> ErrorKind {
        match self.kind() {
            i2c_1_0::ErrorKind::Bus => ErrorKind::I2c(I2cError::Bus),
            i2c_1_0::ErrorKind::NoAcknowledge(_) => ErrorKind::I2c(I2cError::Nack),
            i2c_1_0::ErrorKind::ArbitrationLoss => ErrorKind::I2c(I2cError::ArbitrationLost),
            _ => ErrorKind::Unknown,
        }
    }
}
