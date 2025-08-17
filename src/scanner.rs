/// Scanner utilities for I2C bus device discovery and analysis.
///
/// Supports both embedded-hal 0.2.x and 1.0.x through feature flags:
/// - `ehal_0_2` → uses `blocking::i2c::Write`
/// - `ehal_1_0` → uses `i2c::I2c`
///
/// # Examples
///
/// ```ignore
/// use dvcdbg::logger::{Logger, SerialLogger};
///
/// let mut i2c = /* your i2c interface */;
/// let mut logger = /* your logger */;
///
/// scan_i2c(&mut i2c, &mut logger);
/// scan_i2c_with_ctrl(&mut i2c, &mut logger, &[0x00]);
/// scan_init_sequence(&mut i2c, &mut logger, &[0x00, 0xA5]);
/// ```
use crate::log;
use crate::logger::Logger;
use heapless::Vec;

#[cfg(feature = "ehal_1_0")]
use embedded_hal_1::i2c::I2c;

#[cfg(feature = "ehal_0_2")]
use embedded_hal::blocking::i2c::Write;

/// ==========================
/// I2C 0.2.x implementations
/// ==========================
#[cfg(feature = "ehal_0_2")]
pub fn scan_i2c<I2C, L>(i2c: &mut I2C, logger: &mut L)
where
    I2C: Write<u8>,
    L: Logger,
{
    log!(logger, "[scan] Scanning I2C bus...");

    for addr in 0x03..=0x77 {
        if i2c.write(addr, &[]).is_ok() {
            log!(logger, "[ok] Found device at 0x{:02X}", addr);
        }
    }

    log!(logger, "[info] I2C scan complete.");
}

#[cfg(feature = "ehal_0_2")]
pub fn scan_i2c_with_ctrl<I2C, L>(i2c: &mut I2C, logger: &mut L, control_bytes: &[u8])
where
    I2C: Write<u8>,
    L: Logger,
{
    log!(logger, "[scan] Scanning I2C bus with control bytes: {:?}", control_bytes);

    for addr in 0x03..=0x77 {
        if i2c.write(addr, control_bytes).is_ok() {
            log!(logger, "[ok] Found device at 0x{:02X} (ctrl bytes: {:?})", addr, control_bytes);
        }
    }

    log!(logger, "[info] I2C scan complete.");
}

#[cfg(feature = "ehal_0_2")]
pub fn scan_init_sequence<I2C, L>(i2c: &mut I2C, logger: &mut L, init_sequence: &[u8])
where
    I2C: Write<u8>,
    L: Logger,
{
    log!(logger, "[scan] Scanning I2C bus with init sequence: {:02X?}", init_sequence);

    let mut detected_cmds = Vec::<u8, 64>::new();

    for &cmd in init_sequence {
        log!(logger, "-> Testing command 0x{:02X}", cmd);

        for addr in 0x03..=0x77 {
            if i2c.write(addr, &[0x00, cmd]).is_ok() {
                log!(logger, "[ok] Found device at 0x{:02X} responding to command 0x{:02X}", addr, cmd);
            }
        }

        if detected_cmds.push(cmd).is_err() {
            log!(logger, "[warn] Detected commands buffer is full, results may be incomplete!");
        }
    }

    log!(logger, "Expected sequence: {:02X?}", init_sequence);
    log!(logger, "Commands with response: {:02X?}", detected_cmds.as_slice());

    let mut missing_cmds: Vec<u8, 64> = Vec::new();
    for &cmd in init_sequence {
        if detected_cmds.iter().all(|&d| d != cmd) {
            let _ = missing_cmds.push(cmd);
        }
    }
    log!(logger, "Commands with no response: {:02X?}", missing_cmds.as_slice());
    log!(logger, "[info] I2C scan with init sequence complete.");
}

/// ==========================
/// I2C 1.0.x implementations
/// ==========================
#[cfg(feature = "ehal_1_0")]
pub fn scan_i2c<I2C, L>(i2c: &mut I2C, logger: &mut L)
where
    I2C: I2c,
    I2C::Error: core::fmt::Debug,
    L: Logger,
{
    log!(logger, "[scan] Scanning I2C bus...");

    for addr in 0x03..=0x77 {
        if i2c.write(addr, &[]).is_ok() {
            log!(logger, "[ok] Found device at 0x{:02X}", addr);
        }
    }

    log!(logger, "[info] I2C scan complete.");
}

#[cfg(feature = "ehal_1_0")]
pub fn scan_i2c_with_ctrl<I2C, L>(i2c: &mut I2C, logger: &mut L, control_bytes: &[u8])
where
    I2C: I2c,
    I2C::Error: core::fmt::Debug,
    L: Logger,
{
    log!(logger, "[scan] Scanning I2C bus with control bytes: {:?}", control_bytes);

    for addr in 0x03..=0x77 {
        if i2c.write(addr, control_bytes).is_ok() {
            log!(logger, "[ok] Found device at 0x{:02X} (ctrl bytes: {:?})", addr, control_bytes);
        }
    }

    log!(logger, "[info] I2C scan complete.");
}

#[cfg(feature = "ehal_1_0")]
pub fn scan_init_sequence<I2C, L>(i2c: &mut I2C, logger: &mut L, init_sequence: &[u8])
where
    I2C: I2c,
    I2C::Error: core::fmt::Debug,
    L: Logger,
{
    log!(logger, "[scan] Scanning I2C bus with init sequence: {:02X?}", init_sequence);

    let mut detected_cmds = Vec::<u8, 64>::new();

    for &cmd in init_sequence {
        log!(logger, "-> Testing command 0x{:02X}", cmd);

        for addr in 0x03..=0x77 {
            if i2c.write(addr, &[0x00, cmd]).is_ok() {
                log!(logger, "[ok] Found device at 0x{:02X} responding to command 0x{:02X}", addr, cmd);
            }
        }

        if detected_cmds.push(cmd).is_err() {
            log!(logger, "[warn] Detected commands buffer is full, results may be incomplete!");
        }
    }

    log!(logger, "Expected sequence: {:02X?}", init_sequence);
    log!(logger, "Commands with response: {:02X?}", detected_cmds.as_slice());

    let mut missing_cmds: Vec<u8, 64> = Vec::new();
    for &cmd in init_sequence {
        if detected_cmds.iter().all(|&d| d != cmd) {
            let _ = missing_cmds.push(cmd);
        }
    }
    log!(logger, "Commands with no response: {:02X?}", missing_cmds.as_slice());
    log!(logger, "[info] I2C scan with init sequence complete.");
}