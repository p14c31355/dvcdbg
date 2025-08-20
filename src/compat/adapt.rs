pub struct CoreWriteAdapter<W>(pub W);

impl<W> core::fmt::Write for CoreWriteAdapter<W>
where
    W: embedded_io::Write,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        W::write_all(&mut self.0, s.as_bytes()).map_err(|_| core::fmt::Error)
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

pub trait AdaptWrite {
    type Error;
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error>;
    fn flush(&mut self) -> Result<(), Self::Error>;
}
