//! src/compat/adapt.rs
use core::fmt;
use crate::compat::serial_compat::SerialCompat;
use crate::compat::err_compat::HalErrorExt;
use crate::error::ErrorKind;

pub struct FmtWriteAdapter<T: SerialCompat> {
    inner: T,
    pub last_error: Option<T::Error>,
}

impl<T: SerialCompat> FmtWriteAdapter<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, last_error: None }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn take_last_error(&mut self) -> Option<T::Error> {
        self.last_error.take()
    }

    pub fn take_last_error_kind(&mut self) -> Option<ErrorKind>
    where
        T::Error: HalErrorExt,
    {
        self.last_error.take().map(|e| e.to_compat(None))
    }
}

impl<T: SerialCompat> fmt::Write for FmtWriteAdapter<T> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Err(e) = self.inner.write(s.as_bytes()) {
            self.last_error = Some(e);
            return Err(fmt::Error);
        }
        Ok(())
    }
}
