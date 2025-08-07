use embedded_hal::i2c::I2c;
use crate::logger::*;

/// Scan the I2C bus for connected devices (0x03 to 0x77).
pub fn scan_i2c<I2C, E>(i2c: &mut I2C)
where
    I2C: I2c<Error = E>,
{
    log!("🔍 Scanning I2C bus...");
    for addr in 0x03..=0x77 {
        if i2c.write(addr, &[]).is_ok() {
            log!("✅ Found device at 0x{:02X}", addr);
        }
    }
    log!("🛑 I2C scan complete.");
}
