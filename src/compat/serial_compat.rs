//! serial_compat.rs
use core::fmt::Debug;
use nb;
use embedded_io;

/// common Serial Write trait
pub trait SerialCompat {
    type Error: Debug;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error>;
    fn flush(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub struct CompatErr<E>(pub E);

impl<E: Debug> embedded_io::Error for CompatErr<E> {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

impl<S: SerialCompat> embedded_io::Write for S {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for byte in buf {
            SerialCompat::write(self, *byte)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        SerialCompat::flush(self)
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

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        nb::block!(embedded_hal_0_2::serial::Write::write(self, byte))
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        nb::block!(embedded_hal_0_2::serial::Write::flush(self))
    }
}

// ========== ehal 1.0 ==========
#[cfg(feature = "ehal_1_0")]
impl<S> SerialCompat for S
where
    S: embedded_hal_1::serial::nb::Write<u8>,
    <S as embedded_hal_1::serial::nb::Write<u8>>::Error: Debug + Copy,
{
    type Error = <S as embedded_hal_1::serial::nb::Write<u8>>::Error;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        nb::block!(embedded_hal_1::serial::nb::Write::write(self, byte))
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        nb::block!(embedded_hal_1::serial::nb::Write::flush(self))
    }
}
