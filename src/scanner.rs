use embedded_hal::i2c::I2c;
use crate::log;
use crate::logger::Logger;

/// Scan the I2C bus for connected devices (0x03 to 0x77).
pub fn scan_i2c<I2C, E, L>(i2c: &mut I2C, logger: &mut L)
where
    I2C: I2c<Error = E>,
    L: Logger,
{
    log!(logger, "🔍 Scanning I2C bus...");
    for addr in 0x03..=0x77 {
        if i2c.write(addr, &[]).is_ok() {
            log!(logger, "✅ Found device at 0x{:02X}", addr);
        }
    }
    log!(logger, "🛑 I2C scan complete.");
}
