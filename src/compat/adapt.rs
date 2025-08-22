/// Adapter that converts any `embedded_io::Write` into a `core::fmt::Write`.
pub struct CoreWriteAdapter<W>(pub W);

impl<W> core::fmt::Write for CoreWriteAdapter<W>
where
    W: embedded_io::Write,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // Delegate to `embedded_io::Write::write_all`
        self.0.write_all(s.as_bytes()).map_err(|_| core::fmt::Error)
    }
}

impl<W> core::fmt::Debug for CoreWriteAdapter<W>
where
    W: embedded_io::Write,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CoreWriteAdapter").finish()
    }
}

pub struct AddrFmtAdapter<T> {
    inner: T,
}

impl<T> AddrFmtAdapter<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

#[cfg(feature = "ehal_1_0")]
impl<T> core::fmt::Write for AddrFmtAdapter<T>
where
    T: embedded_io::Write,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.as_bytes() {
            self.inner.write(&[*byte]).map_err(|_| core::fmt::Error)?;
        }
        Ok(())
    }
}

#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
impl<T> core::fmt::Write for AddrFmtAdapter<T>
where
    T: embedded_hal_0_2::serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.as_bytes() {
            nb::block!(self.inner.write(*byte)).map_err(|_| fmt::Error)?;
        }
        Ok(())
    }
}

use core::fmt;

pub struct SerialErrorWrapper<E> {
    pub error: E,
}

impl<E: fmt::Debug> fmt::Display for SerialErrorWrapper<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HAL error: {:?}", self.error)
    }
}

pub struct SerialAdapter<T> {
    inner: T,
}

impl<T> SerialAdapter<T>
where
    T: embedded_io::Write,
{
    pub fn write_bytes(&mut self, buf: &[u8]) -> Result<usize, T::Error> {
        self.inner.write(buf)
    }

    pub fn flush(&mut self) -> Result<(), T::Error> {
        self.inner.flush()
    }
}
