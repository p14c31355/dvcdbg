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
impl<I2C, E> I2cCompat for I2C
where
    I2C: embedded_hal_0_2::blocking::i2c::Write<Error = E>
        + embedded_hal_0_2::blocking::i2c::Read<Error = E>
        + embedded_hal_0_2::blocking::i2c::WriteRead<Error = E>,
    E: Debug + Copy,
{
    type Error = E;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        embedded_hal_0_2::blocking::i2c::Write::write(self, addr, bytes)
    }

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        embedded_hal_0_2::blocking::i2c::Read::read(self, addr, buffer)
    }

    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        embedded_hal_0_2::blocking::i2c::WriteRead::write_read(self, addr, bytes, buffer)
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

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        embedded_hal_1::i2c::I2c::read(self, addr, buffer)
    }

    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        embedded_hal_1::i2c::I2c::write_read(self, addr, bytes, buffer)
    }
}
