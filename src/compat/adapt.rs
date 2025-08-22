//! src/compat/adapt.rs
use core::fmt;
use crate::compat::serial_compat::SerialCompat;

pub struct FmtWriteAdapter<T: SerialCompat> {
    inner: T,
    pub last_error: Option<T::Error>,
}

impl<T> FmtWriteAdapter<T: SerialCompat> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn take_last_error(&mut self) -> Option<T::Error> {
        self.last_error.take()
    }
}

#[cfg(feature = "ehal_1_0")]
impl<T: SerialCompat> fmt::Write for FmtWriteAdapter<T>
where
    T: embedded_io::Write,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Err(e) = self.into_inner().write(s.as_bytes()) {
            self.take_last_error() -> Some(e);
            return Err(fmt::Error);
        }
        Ok(())
    }
}

#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
impl<T> fmt::Write for FmtWriteAdapter<T>
where
    T: embedded_hal_0_2::serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            nb::block!(self.inner.write(b)).map_err(|_| fmt::Error)?;
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
