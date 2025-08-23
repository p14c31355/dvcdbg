//! src/compat/i2c_compat.rs
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
    E: Debug,
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

#[cfg(test)]
mod tests {
    use super::*;

    // ===== 1.0 Dummy I2C =====
    #[cfg(feature = "ehal_1_0")]
    mod ehal_1_0_tests {
        use super::*;
        use embedded_hal_1::i2c::{I2c, ErrorType, Operation};
        #[derive(Debug)]
        struct DummyI2c;

        impl ErrorType for DummyI2c {
            type Error = core::convert::Infallible;
        }

        impl I2c for DummyI2c {
            fn write(&mut self, _addr: u8, _bytes: &[u8]) -> Result<(), Self::Error> { Ok(()) }
            fn read(&mut self, _addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
                for b in buffer.iter_mut() { *b = 0xAA; }
                Ok(())
            }
            fn write_read(&mut self, _addr: u8, _bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
                for b in buffer.iter_mut() { *b = 0x55; }
                Ok(())
            }
            fn transaction(&mut self, _addr: u8, _ops: &mut [Operation<'_>]) -> Result<(), Self::Error> {
                Ok(())
            }
        }

        #[test]
        fn test_dummy_i2c() {
            let mut i2c = DummyI2c;
            let mut buf = [0u8; 4];

            assert!(I2c::write(&mut i2c, 0x42, &[1,2,3]).is_ok());
            assert!(I2c::read(&mut i2c, 0x42, &mut buf).is_ok());
            assert_eq!(buf, [0xAA; 4]);

            assert!(I2c::write_read(&mut i2c, 0x42, &[9], &mut buf).is_ok());
            assert_eq!(buf, [0x55; 4]);

            let mut ops = [];
            assert!(i2c.transaction(0x42, &mut ops).is_ok());
        }
    }

    // ===== 0.2 Dummy I2C =====
    #[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
    mod ehal_0_2_tests {
        use super::*;
        use embedded_hal_0_2::blocking::i2c::{Write, Read, WriteRead};

        #[derive(Debug)]
        struct DummyI2c;

        impl Write for DummyI2c {
            type Error = core::convert::Infallible;
            fn write(&mut self, _addr: u8, _bytes: &[u8]) -> Result<(), Self::Error> { Ok(()) }
        }

        impl Read for DummyI2c {
            type Error = core::convert::Infallible;
            fn read(&mut self, _addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
                for b in buffer.iter_mut() { *b = 0xAA; }
                Ok(())
            }
        }

        impl WriteRead for DummyI2c {
            type Error = core::convert::Infallible;
            fn write_read(&mut self, _addr: u8, _bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
                for b in buffer.iter_mut() { *b = 0x55; }
                Ok(())
            }
        }

        #[test]
        fn test_i2c_write_read_0_2() {
            let mut i2c = DummyI2c;
            let mut buf = [0u8; 4];

            assert!(i2c.write(0x42, &[1,2,3]).is_ok());
            assert!(i2c.read(0x42, &mut buf).is_ok());
            assert_eq!(buf, [0xAA; 4]);

            assert!(i2c.write_read(0x42, &[9], &mut buf).is_ok());
            assert_eq!(buf, [0x55; 4]);
        }
    }
}
