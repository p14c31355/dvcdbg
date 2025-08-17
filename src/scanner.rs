//! Scanner utilities for I2C bus device discovery and analysis.
//!
//! Supports both embedded-hal 0.2.x and 1.0.x through feature flags:
//! - `ehal_0_2` → uses `blocking::i2c::Write`
//! - `ehal_1_0` → uses `i2c::I2c`
//!
//! # Examples
//!
//! ```ignore
//! use dvcdbg::logger::{Logger, SerialLogger};
//!
//! let mut i2c = /* your i2c interface */;
//! let mut logger = /* your logger */;
//!
//! scan_i2c(&mut i2c, &mut logger);
//! scan_i2c_with_ctrl(&mut i2c, &mut logger, &[0x00]);
//! scan_init_sequence(&mut i2c, &mut logger, &[0x00, 0xA5]);
//! ```
use crate::log;
use crate::logger::Logger;
use heapless::Vec;

#[cfg(feature = "ehal_1_0")]
use embedded_hal_1::i2c::I2c;

#[cfg(feature = "ehal_0_2")]
use embedded_hal::blocking::i2c::Write;

/// Scan the I2C bus for connected devices (addresses 0x03 to 0x77).
pub fn scan_i2c<I2C, L>(i2c: &mut I2C, logger: &mut L)
where
    L: Logger,
{
    log!(logger, "[scan] Scanning I2C bus...");

    for addr in 0x03..=0x77 {
        let ok = {
            #[cfg(feature = "ehal_1_0")]
            {
                embedded_hal_1::i2c::I2c::write(i2c, addr, &[]).is_ok()
            }

            #[cfg(feature = "ehal_0_2")]
            {
                Write::write(i2c, addr, &[]).is_ok()
            }
        };

        if ok {
            log!(logger, "[ok] Found device at 0x{:02X}", addr);
        }
    }

    log!(logger, "[info] I2C scan complete.");
}

/// Scan the I2C bus with specified control bytes.
pub fn scan_i2c_with_ctrl<I2C, L>(i2c: &mut I2C, logger: &mut L, control_bytes: &[u8])
where
    L: Logger,
{
    log!(logger, "[scan] Scanning I2C bus with control bytes: {:?}", control_bytes);

    for addr in 0x03..=0x77 {
        let ok = {
            #[cfg(feature = "ehal_1_0")]
            {
                embedded_hal_1::i2c::I2c::write(i2c, addr, control_bytes).is_ok()
            }

            #[cfg(feature = "ehal_0_2")]
            {
                Write::write(i2c, addr, control_bytes).is_ok()
            }
        };

        if ok {
            log!(
                logger,
                "[ok] Found device at 0x{:02X} (ctrl bytes: {:?})",
                addr,
                control_bytes
            );
        }
    }

    log!(logger, "[info] I2C scan complete.");
}

/// Scan the I2C bus by testing an initialization sequence.
pub fn scan_init_sequence<I2C, L>(i2c: &mut I2C, logger: &mut L, init_sequence: &[u8])
where
    L: Logger,
{
    log!(logger, "[scan] Scanning I2C bus with init sequence: {:02X?}", init_sequence);

    let mut detected_cmds = Vec::<u8, 64>::new();

    for &cmd in init_sequence {
        log!(logger, "-> Testing command 0x{:02X}", cmd);

        for addr in 0x03..=0x77 {
            let ok = {
                #[cfg(feature = "ehal_1_0")]
                {
                    embedded_hal_1::i2c::I2c::write(i2c, addr, &[0x00, cmd]).is_ok()
                }

                #[cfg(feature = "ehal_0_2")]
                {
                    Write::write(i2c, addr, &[0x00, cmd]).is_ok()
                }
            };

            if ok {
                log!(
                    logger,
                    "[ok] Found device at 0x{:02X} responding to command 0x{:02X}",
                    addr,
                    cmd
                );
            }
        }

        if detected_cmds.push(cmd).is_err() {
            log!(
                logger,
                "[warn] Detected commands buffer is full, results may be incomplete!"
            );
        }
    }

    log!(logger, "Expected sequence: {:02X?}", init_sequence);
    log!(logger, "Commands with response: {:02X?}", detected_cmds.as_slice());

    detected_cmds.sort_unstable();
    let missing_cmds: Vec<u8, 64> = init_sequence
        .iter()
        .filter(|&&c| detected_cmds.binary_search(&c).is_err())
        .copied()
        .collect();

    log!(logger, "Commands with no response: {:02X?}", missing_cmds.as_slice());
    log!(logger, "[info] I2C scan with init sequence complete.");
}