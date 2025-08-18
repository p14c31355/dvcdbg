//! Scanner utilities for I2C bus device discovery and analysis.
//!
//! This module provides functions to scan the I2C bus for connected devices,
//! optionally testing with control bytes or initialization command sequences,
//! with detailed logging support.

use crate::log;
use crate::logger::Logger;
use embedded_hal::i2c::I2c;
use heapless::Vec;

// -----------------------------------------------------------------------------
//  Public API (with Rustdoc) 
// -----------------------------------------------------------------------------

/// Scan the I2C bus for connected devices (addresses `0x03` to `0x77`).
///
/// This function probes each possible I2C device address by attempting to
/// write an empty buffer (`[]`). Devices that acknowledge are reported
/// through the provided logger.
///
/// # Arguments
///
/// * `i2c` - Mutable reference to the I2C interface implementing [`embedded_hal::i2c::I2c`].
/// * `logger` - Mutable reference to a logger implementing the [`Logger`] trait.
///
/// # Example
///
/// ```ignore
/// use embedded_hal::i2c::I2c;
/// use dvcdbg::logger::SerialLogger;
/// use dvcdbg::scanner::scan_i2c;
///
/// let mut i2c = /* your i2c interface */;
/// let mut logger = SerialLogger::new(/* serial */);
///
/// scan_i2c(&mut i2c, &mut logger);
/// ```
pub fn scan_i2c<I2C, L>(i2c: &mut I2C, logger: &mut L)
where
    I2C: I2c,
    L: Logger,
{
    log!(logger, "[scan] Scanning I2C bus...");
    let found_addrs = internal_scan(i2c, &[]);
    for addr in found_addrs {
        log!(logger, "[ok] Found device at 0x{:02X}", addr);
    }
    log!(logger, "[info] I2C scan complete.");
}

/// Scan the I2C bus for devices by sending specified control bytes.
///
/// This variant allows specifying control bytes (e.g., `0x00`) to send
/// alongside the probe. Devices that acknowledge the transmission are
/// reported via the logger.
///
/// # Arguments
///
/// * `i2c` - Mutable reference to the I2C interface implementing [`embedded_hal::i2c::I2c`].
/// * `logger` - Mutable reference to a logger implementing the [`Logger`] trait.
/// * `control_bytes` - Slice of bytes to send when probing each device.
///
/// # Example
///
/// ```ignore
/// use embedded_hal::i2c::I2c;
/// use dvcdbg::logger::SerialLogger;
/// use dvcdbg::scanner::scan_i2c_with_ctrl;
///
/// let mut i2c = /* your i2c interface */;
/// let mut logger = SerialLogger::new(/* serial */);
///
/// scan_i2c_with_ctrl(&mut i2c, &mut logger, &[0x00]);
/// ```
pub fn scan_i2c_with_ctrl<I2C, L>(i2c: &mut I2C, logger: &mut L, control_bytes: &[u8])
where
    I2C: I2c,
    L: Logger,
{
    log!(
        logger,
        "[scan] Scanning I2C bus with control bytes: {:?}",
        control_bytes
    );
    let found_addrs = internal_scan(i2c, control_bytes);
    for addr in found_addrs {
        log!(
            logger,
            "[ok] Found device at 0x{:02X} (ctrl bytes: {:?})",
            addr,
            control_bytes
        );
    }
    log!(logger, "[info] I2C scan complete.");
}

/// Scan the I2C bus using an initialization sequence of commands.
///
/// Each command in the sequence is transmitted to all possible device
/// addresses using the control byte `0x00`. The function records which
/// commands receive responses and highlights any **differences** between
/// the expected and observed responses.
///
/// This is useful for verifying whether a device supports the expected
/// initialization commands (e.g., when testing display controllers).
///
/// # Arguments
///
/// * `i2c` - Mutable reference to the I2C interface implementing [`embedded_hal::i2c::I2c`].
/// * `logger` - Mutable reference to a logger implementing the [`Logger`] trait.
/// * `init_sequence` - Slice of initialization commands to test.
///
/// # Example
///
/// ```ignore
/// use embedded_hal::i2c::I2c;
/// use dvcdbg::logger::SerialLogger;
/// use dvcdbg::scanner::scan_init_sequence;
///
/// let mut i2c = /* your i2c interface */;
/// let mut logger = SerialLogger::new(/* serial */);
///
/// let init_sequence: [u8; 3] = [0xAE, 0xA1, 0xAF]; // example init cmds
/// scan_init_sequence(&mut i2c, &mut logger, &init_sequence);
/// ```
pub fn scan_init_sequence<I2C, L>(i2c: &mut I2C, logger: &mut L, init_sequence: &[u8])
where
    I2C: I2c,
    L: Logger,
{
    log!(
        logger,
        "[scan] Scanning I2C bus with init sequence: {:02X?}",
        init_sequence
    );
    let mut detected_cmds = Vec::<u8, 64>::new();

    for &cmd in init_sequence {
        log!(logger, "-> Testing command 0x{:02X}", cmd);
        let found_addrs = internal_scan(i2c, &[0x00, cmd]);

        if !found_addrs.is_empty() {
            for addr in found_addrs {
                log!(
                    logger,
                    "[ok] Found device at 0x{:02X} responding to command 0x{:02X}",
                    addr,
                    cmd
                );
            }
            if detected_cmds.push(cmd).is_err() {
                log!(
                    logger,
                    "[warn] Detected commands buffer is full, results may be incomplete!"
                );
            }
        }
    }

    log_differences(logger, init_sequence, &detected_cmds);
    log!(logger, "[info] I2C scan with init sequence complete.");
}

// -----------------------------------------------------------------------------
//  Internal utilities (not exported as public API) 
// -----------------------------------------------------------------------------

fn internal_scan<I2C>(i2c: &mut I2C, data: &[u8]) -> heapless::Vec<u8, 128>
where
    I2C: I2c,
{
    let mut found_devices = heapless::Vec::new();
    for addr in 0x03..=0x77 {
        if i2c.write(addr, data).is_ok() {
            // The push cannot fail because the address range (0x03..=0x77) has 120
            // possible addresses, and the vector's capacity is 128.
            let _ = found_devices.push(addr);
        }
    }
    found_devices
}

fn log_differences<L>(logger: &mut L, expected: &[u8], mut detected: Vec<u8, 64>)
where
    L: Logger,
{
    log!(logger, "Expected sequence: {:02X?}", expected);
    log!(logger, "Commands with response: {:02X?}", detected.as_slice());

    let mut sorted = detected.clone();
    sorted.sort_unstable();
    let mut missing_cmds: Vec<u8, 64> = Vec::new();
    for cmd in expected.iter().copied().filter(|c| sorted.binary_search(c).is_err()) {
        if missing_cmds.push(cmd).is_err() {
            log!(logger, "[warn] Missing commands buffer is full, list is truncated.");
            break;
        }
    }
    log!(
        logger,
        "Commands with no response: {:02X?}",
        missing_cmds.as_slice()
    );
}
