//! serial_compat.rs
use core::fmt::Debug;
#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
use nb;
use embedded_io;

/// common Serial Write trait
/// The `write` method is now slice-oriented.
pub trait SerialCompat {
    type Error: embedded_io::Error + Debug;

    fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error>;
    fn flush(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub struct CompatErr<E>(pub E);

impl<E: Debug> embedded_io::Error for CompatErr<E> {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

// ========== ehal 1.0 ==========
#[cfg(feature = "ehal_1_0")]
impl<S> SerialCompat for S
where
    S: embedded_io::Write,
    <S as embedded_io::ErrorType>::Error: Debug + Copy,
{
    type Error = <S as embedded_io::ErrorType>::Error;

    fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        embedded_io::Write::write_all(self, buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        embedded_io::Write::flush(self)
    }
}

// ========== ehal 0.2.x ==========
#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
impl<S> SerialCompat for S
where
    S: embedded_hal_0_2::serial::Write<u8>,
    <S as embedded_hal_0_2::serial::Write<u8>>::Error: Debug + Copy,
{
    type Error = <S as embedded_hal_0_2::serial::Write<u8>>::Error;

    fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        for byte in buf {
            nb::block!(embedded_hal_0_2::serial::Write::write(self, *byte))?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        nb::block!(self.flush())
    }
}