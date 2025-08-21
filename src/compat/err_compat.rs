// src/compat/err_compat.rs
use crate::error::ErrorKind;

/// Trait to adapt HAL-specific error types into our domain error.
pub trait ErrorCompat {
    fn to_kind(&self) -> ErrorKind;
}

#[cfg(feature = "ehal_1_0")]
impl ErrorCompat for embedded_hal_1::i2c::ErrorKind {
    fn to_kind(&self) -> ErrorKind {
        match self {
            embedded_hal_1::i2c::ErrorKind::NoAcknowledge => ErrorKind::I2cNack,
            embedded_hal_1::i2c::ErrorKind::Bus => ErrorKind::I2cBus,
            _ => ErrorKind::Unknown,
        }
    }
}
