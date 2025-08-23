use dvcdbg::prelude::*;
use dvcdbg::compat::{SerialCompat, I2cCompat};

// -----------------------------
// Dummy implementations
// -----------------------------
struct DummySerial;
impl SerialCompat for DummySerial {
    type Error = core::convert::Infallible;
    fn write(&mut self, _buf: &[u8]) -> Result<(), Self::Error> { Ok(()) }
    fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
}

struct DummyI2c;
impl I2cCompat for DummyI2c {
    type Error = core::convert::Infallible;
    fn write(&mut self, _addr: u8, _buf: &[u8]) -> Result<(), Self::Error> { Ok(()) }
    fn read(&mut self, _addr: u8, _buf: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
    fn write_read(&mut self, _addr: u8, _buf: &[u8], _out: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
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
    assert!(i2c.write(0x42, &[1,2,3]).is_ok());
    let mut buf = [0u8; 3];
    assert!(i2c.read(0x42, &mut buf).is_ok());
    assert!(i2c.write_read(0x42, &[1,2], &mut buf).is_ok());

    scan_i2c(&mut i2c, &mut DummySerial);

    assert_log!(serial, "test log macro");
    quick_diag!(serial, "dummy diag");

    write_bin!(serial, b"\x00\xFF").unwrap();
    write_hex!(serial, b"\xAA\xBB").unwrap();
}
