use dvcdbg::compat::{SerialCompat, I2cCompat};

// Dummy implementations
struct DummyI2c;
struct DummySerial;

impl I2cCompat for DummyI2c {
    type Error = core::convert::Infallible;
    fn write(&mut self, _addr: u8, _buf: &[u8]) -> Result<(), Self::Error> { Ok(()) }
    fn read(&mut self, _addr: u8, _buf: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
    fn write_read(&mut self, _addr: u8, _buf: &[u8], _out: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
}

impl SerialCompat for DummySerial {
    type Error = core::convert::Infallible;
    fn write(&mut self, _buf: &[u8]) -> Result<(), Self::Error> { Ok(()) }
    fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
}

#[test]
fn test_full_stack() {
    let mut i2c = DummyI2c;
    let mut serial = DummySerial;

    // I2C write/read
    assert!(i2c.write(0x42, &[1,2,3]).is_ok());
    let mut buf = [0;3];
    assert!(i2c.read(0x42, &mut buf).is_ok());

    // Serial write/flush
    assert!(serial.write(b"hello").is_ok());
    assert!(serial.flush().is_ok());
}
