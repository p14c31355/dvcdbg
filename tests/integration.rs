use core::fmt::Write;
use dvcdbg::compat::{I2cCompat, SerialCompat};
use dvcdbg::prelude::*;

// -----------------------------
// Dummy implementations
// -----------------------------
struct DummySerial;
impl SerialCompat for DummySerial {
    type Error = core::convert::Infallible;

    fn write(&mut self, _buf: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl core::fmt::Write for DummySerial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // For testing purposes, we can print to stdout to see what would be logged
        print!("{}", s);
        Ok(())
    }
}


struct DummyI2c;
impl I2cCompat for DummyI2c {
    type Error = core::convert::Infallible;

    fn write(&mut self, _addr: u8, _bytes: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }
    fn read(&mut self, _addr: u8, _buffer: &mut [u8]) -> Result<(), Self::Error> {
        Ok(())
    }
    fn write_read(
        &mut self,
        _addr: u8,
        _bytes: &[u8],
        _buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

// -----------------------------
// Integration test
// -----------------------------
#[test]
fn test_full_stack() {
    // Serial test
    let mut serial = DummySerial;
    assert!(serial.write(b"hello").is_ok());
    assert!(serial.flush().is_ok());

    // I2C test
    let mut i2c = DummyI2c;
    assert!(i2c.write(0x42, &[1, 2, 3]).is_ok());
    let mut buf = [0u8; 3];
    assert!(i2c.read(0x42, &mut buf).is_ok());
    assert!(i2c.write_read(0x42, &[1, 2], &mut buf).is_ok());

    assert!(scan_i2c(&mut i2c, &mut serial, 0x00).is_ok());

    assert_log!(false, &mut serial, "test log macro");

    assert_log!(true, &mut serial, "this won't log");

    quick_diag!(&mut serial, &mut i2c);

    write_bin!(&mut serial, &[0x00, 0xFF]);
    write_hex!(&mut serial, &[0xAA, 0xBB]);
}
