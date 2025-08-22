//! src/compat/adapt.rs
use core::fmt;
use crate::compat::serial_compat::SerialCompat;

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

pub struct SerialErrorWrapper<E>(pub E);

impl<E: fmt::Debug> fmt::Display for SerialErrorWrapper<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HAL error: {:?}", self.0)
    }
}
