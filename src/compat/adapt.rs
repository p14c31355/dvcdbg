//! src/compat/adapt.rs
//! Adapter that wraps any `SerialCompat` implementor and exposes a `core::fmt::Write`
//! interface while retaining the original HAL error for later inspection.

use core::fmt;
use crate::compat::serial_compat::SerialCompat;
use crate::compat::err_compat::HalErrorExt;
use crate::error::ErrorKind;

/// A lightweight adapter to write formatted strings to a HAL serial/I2C interface.
///
/// This adapter allows using `write!` and `writeln!` macros on any
/// `SerialCompat` implementor, while storing the last underlying HAL error
/// for later inspection.
///
/// # Example
///
/// ```ignore
/// # use dvcdbg::compat::adapt::FmtWriteAdapter;
/// # use dvcdbg::compat::serial_compat::SerialCompat;
/// # struct DummySerial;
/// # impl SerialCompat for DummySerial {
/// #     type Error = ();
/// #     fn write(&mut self, _buf: &[u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
/// # }
/// let mut uart = FmtWriteAdapter::new(DummySerial);
/// writeln!(uart, "Hello, world!").ok();
/// ```
pub struct FmtWriteAdapter<T: SerialCompat> {
    inner: T,
    /// Stores the last HAL error encountered during write.
    pub last_error: Option<T::Error>,
}

impl<T: SerialCompat> FmtWriteAdapter<T> {
    /// Create a new adapter wrapping a serial device.
    pub fn new(inner: T) -> Self {
        Self { inner, last_error: None }
    }

    /// Extract the inner serial device, consuming the adapter.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Take the last HAL error, if any.
    pub fn take_last_error(&mut self) -> Option<T::Error> {
        self.last_error.take()
    }

    /// Convert the last HAL error into a unified `ErrorKind`.
    pub fn take_last_error_kind(&mut self) -> Option<ErrorKind>
    where
        T::Error: HalErrorExt,
    {
        self.last_error.take().map(|e| e.to_compat(None))
    }
}

impl<T: SerialCompat> fmt::Write for FmtWriteAdapter<T> {
    /// Write a string slice to the underlying serial device.
    ///
    /// On HAL write error, stores the error in `last_error` and returns `fmt::Error`.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Err(e) = self.inner.write(s.as_bytes()) {
            self.last_error = Some(e);
            return Err(fmt::Error);
        }
        Ok(())
    }
}
