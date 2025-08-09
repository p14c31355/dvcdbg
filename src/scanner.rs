use embedded_hal::i2c::I2c;
use crate::log;
use crate::logger::Logger;

/// Scan the I2C bus for connected devices (0x03 to 0x77).
pub fn scan_i2c<I2C, L>(i2c: &mut I2C, logger: &mut L)
where
    I2C: I2c,
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

/// Scan the I2C bus for devices, testing write with optional control bytes.
pub fn scan_i2c_with_ctrl<I2C, L>(i2c: &mut I2C, logger: &mut L, control_bytes: &[u8])
where
    I2C: I2c,
    L: Logger,
{
    log!(logger, "🔍 Scanning I2C bus with control bytes: {:?}", control_bytes);
    for addr in 0x03..=0x77 {
        if i2c.write(addr, control_bytes).is_ok() {
            log!(logger, "✅ Found device at 0x{:02X} (ctrl bytes: {:?})", addr, control_bytes);
        }
    }
    log!(logger, "🛑 I2C scan complete.");
}

use heapless::Vec;

pub fn scan_init_sequence<I2C, L>(
    i2c: &mut I2C,
    logger: &mut L,
    init_sequence: &[u8],
) where
    I2C: I2c,
    L: Logger,
{
    log!(logger, "🔍 Scanning I2C bus with init sequence: {:02X?}", init_sequence);

    let mut detected_cmds = Vec::<u8, 64>::new();

    for &cmd in init_sequence {
        log!(logger, "→ Testing command 0x{:02X}", cmd);
        let mut found_on_any = false;

        for addr in 0x03..=0x77 {
            let res = i2c.write(addr, &[0x00, cmd]); // 0x00 = control byte for command
            if res.is_ok() {
                log!(logger, "✅ Found device at 0x{:02X} responding to command 0x{:02X}", addr, cmd);
                found_on_any = true;
            }
        }

        if detected_cmds.push(cmd).is_err() {
            log!(logger, "⚠️ Detected commands buffer is full, results may be incomplete!");
        }
    }

    // 差分表示
    log!(logger, "Expected sequence: {:02X?}", init_sequence);
    log!(logger, "Commands with response: {:02X?}", detected_cmds.as_slice());

    detected_cmds.sort_unstable();
    let missing_cmds: Vec<u8, 64> = init_sequence.iter()
        .filter(|&&c| detected_cmds.binary_search(&c).is_err())
        .copied()
        .collect();

    log!(logger, "Commands with no response: {:02X?}", missing_cmds.as_slice());

    log!(logger, "🛑 I2C scan with init sequence complete.");
}
