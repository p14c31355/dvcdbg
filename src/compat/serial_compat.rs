//! src/compat/serial_compat.rs
use core::fmt::Debug;
use embedded_io;
#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
use nb;
/// ### Differ bus injection with blanket (SELF RESPONSIBILITY)
/// ```ignore
/// use dvcdbg::prelude::*;
///
/// struct MyUart;
/// impl embedded_io::Write for MyUart {
///    type Error = core::convert::Infallible;
///    fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> { Ok(()) }
///    fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
/// }
///
/// impl UartLike for MyUart {}
///
/// let mut serial = SerialEio(MyUart);
/// serial.write(b"hello")?;
///```
pub trait UartLike: embedded_io::Write {}

#[derive(Debug)]
pub struct SerialEio<S: UartLike>(pub S);

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
impl<S> SerialCompat for SerialEio<S>
where
    S: UartLike,
    <S as embedded_io::ErrorType>::Error: Debug,
{
    type Error = <S as embedded_io::ErrorType>::Error;

    fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        embedded_io::Write::write_all(&mut self.0, buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        embedded_io::Write::flush(&mut self.0)
    }
}

// ========== ehal 0.2.x ==========
#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
impl<S> SerialCompat for S
where
    S: embedded_hal_0_2::serial::Write<u8>,
    <S as embedded_hal_0_2::serial::Write<u8>>::Error: Debug,
{
    type Error = CompatErr<<S as embedded_hal_0_2::serial::Write<u8>>::Error>;

    fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        for byte in buf {
            nb::block!(embedded_hal_0_2::serial::Write::write(self, *byte)).map_err(CompatErr)?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        nb::block!(embedded_hal_0_2::serial::Write::flush(self)).map_err(CompatErr)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== 1.0 Dummy UART =====
    #[cfg(feature = "ehal_1_0")]
    mod ehal_1_0_tests {
        use super::*;

        #[derive(Debug)]
        struct DummyUart;

        impl embedded_io::ErrorType for DummyUart {
            type Error = core::convert::Infallible;
        }

        impl embedded_io::Write for DummyUart {
            fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> Result<(), Self::Error> {
                Ok(())
            }
        }

        impl UartLike for DummyUart {}

        #[test]
        fn test_serial_write_1_0() {
            let mut serial = SerialEio(DummyUart);
            let data = b"hello";

            assert!(serial.write(data).is_ok());
            assert!(serial.flush().is_ok());
        }
    }

    // ===== 0.2 Dummy UART =====
    #[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
    mod ehal_0_2_tests {
        use super::*;
        use embedded_hal_0_2::serial::Write as HalWrite;
        use nb;

        #[derive(Debug)]
        struct DummyUart;

        impl HalWrite<u8> for DummyUart {
            type Error = core::convert::Infallible;

            fn write(&mut self, _word: u8) -> nb::Result<(), Self::Error> {
                Ok(())
            }

            fn flush(&mut self) -> nb::Result<(), Self::Error> {
                Ok(())
            }
        }

        #[test]
        fn test_serial_write_0_2() {
            let mut uart = DummyUart;
            let buf = b"hello";

            for &b in buf {
                assert!(nb::block!(uart.write(b)).is_ok());
            }
            assert!(nb::block!(uart.flush()).is_ok());
        }
    }
}
