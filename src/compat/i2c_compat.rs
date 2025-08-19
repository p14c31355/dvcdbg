use core::fmt::Debug;

/// common I2C trait
pub trait I2cCompat {
    type Error: Debug;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error>;
    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error>;
    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error>;
}

// ========== ehal 0.2.x ==========
#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
impl<I2C> I2cCompat for I2C
where
    I2C: embedded_hal_0_2::blocking::i2c::Write,
    <I2C as embedded_hal_0_2::blocking::i2c::Write>::Error: Debug + Copy,
{
    type Error = <I2C as embedded_hal_0_2::blocking::i2c::Write>::Error;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        embedded_hal_0_2::blocking::i2c::Write::write(self, addr, bytes)
    }
}

// ========== ehal 1.0 ==========
#[cfg(feature = "ehal_1_0")]
impl<I2C> I2cCompat for I2C
where
    I2C: embedded_hal_1::i2c::I2c,
    I2C::Error: Into<embedded_hal_1::i2c::ErrorKind> + Debug + Copy,
{
    type Error = I2C::Error;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        embedded_hal_1::i2c::I2c::write(self, addr, bytes)
    }
}

/// # adapt_i2c! macro
///
/// Wraps an I2C peripheral that implements `embedded-hal::blocking::i2c`
/// (0.2) or `embedded-hal::i2c` (1.0) and provides `I2cCompat`.
///
/// # Example
/// ```ignore
/// adapt_i2c!(avr_i2c: I2cAdapter);
/// ```
#[macro_export]
macro_rules! adapt_i2c {
    ($name:ident : $adapter:ident) => {
        pub struct $adapter<'a>(&'a mut $name);

        impl<'a> $crate::scanner::I2cCompat for $adapter<'a> {
            fn write_read(
                &mut self,
                addr: u8,
                bytes: &[u8],
                buffer: &mut [u8],
            ) -> Result<(), ()> {
                self.0.write_read(addr, bytes, buffer).map_err(|_| ())
            }

            fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), ()> {
                self.0.write(addr, bytes).map_err(|_| ())
            }

            fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), ()> {
                self.0.read(addr, buffer).map_err(|_| ())
            }
        }
    };
}
