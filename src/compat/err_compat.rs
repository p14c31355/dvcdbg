//! Error compatibility layer for embedded-hal 0.2 and 1.0
//! Provides a unified error type for logging and diagnostics in no_std

use core::fmt::Debug;
use crate::error::ErrorKind;
use embedded_hal::i2c;

#[derive(Debug)]
pub enum ErrorCompat {
    Hal(ErrorKind),
    Other(&'static str),
}

impl ErrorCompat {
    pub fn other(msg: &'static str) -> Self {
        ErrorCompat::Other(msg)
    }

    pub fn from_i2c_kind<E>(err: &E) -> Self
    where
        E: i2c::Error,
    {
        match err.kind() {
            i2c::ErrorKind::Bus => ErrorCompat::Hal(ErrorKind::Bus),
            i2c::ErrorKind::ArbitrationLoss => ErrorCompat::Hal(ErrorKind::ArbitrationLoss),
            i2c::ErrorKind::Overrun => ErrorCompat::Hal(ErrorKind::Overrun),
            i2c::ErrorKind::Nack => ErrorCompat::Hal(ErrorKind::Nack),
            _ => ErrorCompat::Hal(ErrorKind::Other),
        }
    }
}

/// Blanket extension trait: usable for all `E: Debug`
pub trait HalErrorExt {
    fn to_compat(&self) -> ErrorCompat;
}

impl<E: Debug> HalErrorExt for E {
    fn to_compat(&self) -> ErrorCompat {
        ErrorCompat::Other("Unknown HAL error")
    }
}

// --- Special helpers for I2C (no trait impl, just free functions) ---
pub fn to_compat_i2c<E>(err: &E) -> ErrorCompat
where
    E: i2c::Error + Debug,
{
    ErrorCompat::from_i2c_kind(err)
}
