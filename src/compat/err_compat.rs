//! Error compatibility layer for embedded-hal 0.2 and 1.0
//! Provides a unified error type for logging and diagnostics in no_std

use core::fmt::Debug;
use crate::error::ErrorKind;

#[derive(Debug)]
pub enum ErrorCompat {
    Hal(ErrorKind),
    Other(&'static str),
}

impl ErrorCompat {
    pub fn other(msg: &'static str) -> Self {
        ErrorCompat::Other(msg)
    }
}

/// Blanket impl for all Debug errors (fallback)
pub trait HalErrorExt {
    fn to_compat(&self) -> ErrorCompat;
}

impl<E: Debug> HalErrorExt for E {
    fn to_compat(&self) -> ErrorCompat {
        ErrorCompat::Other("Unknown HAL error")
    }
}

//
// --- I2C special cases ---
//

// embedded-hal 1.0
#[cfg(feature = "ehal_1_0")]
pub fn to_compat_i2c<E>(err: &E) -> ErrorCompat
where
    E: embedded_hal_1::i2c::Error + Debug,
{
    use embedded_hal_1::i2c::ErrorKind;
    match err.kind() {
        ErrorKind::Bus => ErrorCompat::Hal(crate::error::ErrorKind::I2cBus),
        ErrorKind::ArbitrationLoss => ErrorCompat::Hal(crate::error::ErrorKind::I2cArbitrationLost),
        ErrorKind::Overrun => ErrorCompat::Hal(crate::error::ErrorKind::UartOverrun),
        ErrorKind::Nack => ErrorCompat::Hal(crate::error::ErrorKind::I2cNack),
        _ => ErrorCompat::Hal(crate::error::ErrorKind::Other),
    }
}

// embedded-hal 0.2
#[cfg(feature = "ehal_0_2")]
pub fn to_compat_i2c<E>(_: &E) -> ErrorCompat
where
    E: Debug,
{
    // no structured ErrorKind in 0.2
    ErrorCompat::Other("I2C error (e-hal 0.2)")
}
