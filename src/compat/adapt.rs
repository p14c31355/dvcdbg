//! src/compat/adapt.rs
use core::fmt;

pub struct FmtWriteAdapter<T> {
    inner: T,
}

impl<T> FmtWriteAdapter<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[cfg(feature = "ehal_1_0")]
impl<T> fmt::Write for FmtWriteAdapter<T>
where
    T: embedded_io::Write,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.inner.write_all(s.as_bytes()).map_err(|_| fmt::Error)
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
