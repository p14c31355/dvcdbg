/// Adapter that converts any `embedded_io::Write` into a `core::fmt::Write`.
pub struct CoreWriteAdapter<W>(pub W);

impl<W> core::fmt::Write for CoreWriteAdapter<W>
where
    W: embedded_io::Write,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // `embedded_io::Write::write_all` に委譲
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
