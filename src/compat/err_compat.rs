//!src/compat/err_compat.rs
//! HAL Error Compatibility Layer
//!
//! A wrapper for unifying HAL error types in embedded-hal 0.2 series.
//! Internally it is mapped to a common `ErrorKind` and can be used in the debugger and driver layers.

use core::fmt::Debug;
use crate::error::ErrorKind;

/// Trait to adapt HAL-specific error types into our domain error.
pub trait ErrorCompat {
    fn to_kind(&self) -> ErrorKind;
}

#[cfg(feature = "ehal_1_0")]
impl ErrorCompat for embedded_hal_1::i2c::ErrorKind {
    fn to_kind(&self) -> ErrorKind {
        match self {
            embedded_hal_1::i2c::ErrorKind::NoAcknowledge(_) => ErrorKind::I2cNack,
            embedded_hal_1::i2c::ErrorKind::Bus => ErrorKind::I2cBus,
            _ => ErrorKind::Unknown,
        }
    }
}

/// A compatible type that wraps any HAL-specific errors
#[derive(Debug)]
pub struct HalErrorCompat<E> {
    pub(crate) inner: E,
    pub(crate) kind: ErrorKind,
}

impl<E> HalErrorCompat<E>
where
    E: Debug,
{
    /// HAL エラーを受け取り、ErrorCompat に変換
    pub fn from_hal_error(e: E) -> Self {
        let kind = Self::map_error_kind(&e);
        Self { inner: e, kind }
    }

    /// Mapping from HAL-specific error types to ErrorKind
    fn map_error_kind(_e: &E) -> ErrorKind {
        // TODO: Matching by HAL implementation
        // Default to Other here
        ErrorKind::Other
    }

    /// Internal HAL Error Reference
    pub fn inner(&self) -> &E {
        &self.inner
    }

    /// Common ErrorKind
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

/// Traits that wrap HAL-specific behavior such as I2C/NACK/Serial detection
pub trait HalErrorExt: Debug {
    /// Determine if device is not present (NACK) (I2C)
    fn is_nack(&self) -> bool {
        false
    }
    /// Check whether serial writing is possible (Serial)
    fn is_would_block(&self) -> bool {
        false
    }
}

/// Default implementation of I2C errors in 0.2 series
#[cfg(feature = "ehal_0_2")]
impl<E> HalErrorExt for E
where
    E: Debug,
{
    fn is_nack(&self) -> bool {
        // embedded-hal 0.2 does not have ErrorKind, so it is judged by the Debug string.
        let s = format!("{:?}", self);
        s.contains("NACK") || s.contains("NoAcknowledge")
    }
}

/// Default implementation of I2C errors in 1.0 series
#[cfg(feature = "ehal_1_0")]
impl<E> HalErrorExt for E
where
    E: embedded_hal_1::i2c::Error + Debug,
{
    fn is_nack(&self) -> bool {
        use embedded_hal_1::i2c::ErrorKind;
        matches!(self.kind(), ErrorKind::NoAcknowledge(_))
    }
}

/// Default implementation of embedded-hal 0.2 Serial errors
#[cfg(feature = "ehal_0_2")]
impl<E> HalErrorExt for E
where
    E: nb::ErrorKind + Debug,
{
    fn is_would_block(&self) -> bool {
        matches!(self.kind(), nb::ErrorKind::WouldBlock)
    }
}

/// Macro to convert any HAL error to ErrorCompat
#[macro_export]
macro_rules! hal_err {
    ($e:expr) => {
        $crate::compat::err_compat::ErrorCompat::from_hal_error($e)
    };
}