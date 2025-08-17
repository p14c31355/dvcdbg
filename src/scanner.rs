/// Scanner utilities for I2C bus device discovery and analysis.
///
/// Supports both embedded-hal 0.2.x and 1.0.x through feature flags:
/// - `ehal_0_2` → uses `blocking::i2c::{Write, Read}`
/// - `ehal_1_0` → uses `i2c::I2c`
///
/// # Examples
///
/// ```ignore
/// use dvcdbg::logger::{Logger, SerialLogger};
///
/// #[cfg(feature = "ehal_1_0")]
/// use embedded_hal::i2c::I2c;
///
/// #[cfg(feature = "ehal_0_2")]
/// use embedded_hal::blocking::i2c::Write;
///
/// fn main() {
///     let mut i2c = /* your i2c interface */;
///     let mut logger = /* your logger */;
///
///     scan_i2c(&mut i2c, &mut logger);
///     scan_i2c_with_ctrl(&mut i2c, &mut logger, &[0x00]);
/// }
/// ```
use crate::log;
use crate::logger::Logger;
use heapless::Vec;

#[cfg(feature = "ehal_1_0")]
use embedded_hal::i2c::I2c;

#[cfg(feature = "ehal_0_2")]
use embedded_hal::blocking::i2c::Write;

pub fn scan_i2c<I2C, L>(i2c: &mut I2C, logger: &mut L)
where
    L: Logger,
    I2C: ?Sized,
{
    log!(logger, "[scan] Scanning I2C bus...");

    #[cfg(feature = "ehal_1_0")]
    {
        let i2c = i2c as &mut dyn I2c<Error = I2C::Error>;
        for addr in 0x03..=0x77 {
            if i2c.write(addr, &[]).is_ok() {
                log!(logger, "[ok] Found device at 0x{:02X}", addr);
            }
        }
    }

    #[cfg(feature = "ehal_0_2")]
    {
        let i2c = i2c as &mut dyn Write<Error = I2C::Error>;
        for addr in 0x03..=0x77 {
            if i2c.write(addr, &[]).is_ok() {
                log!(logger, "[ok] Found device at 0x{:02X}", addr);
            }
        }
    }

    log!(logger, "[info] I2C scan complete.");
}

pub fn scan_i2c_with_ctrl<I2C, L>(i2c: &mut I2C, logger: &mut L, control_bytes: &[u8])
where
    L: Logger,
    I2C: ?Sized,
{
    log!(logger, "[scan] Scanning I2C bus with control bytes: {:02X?}", control_bytes);

    #[cfg(feature = "ehal_1_0")]
    {
        let i2c = i2c as &mut dyn I2c<Error = I2C::Error>;
        for addr in 0x03..=0x77 {
            if i2c.write(addr, control_bytes).is_ok() {
                log!(logger, "[ok] Found device at 0x{:02X} (ctrl bytes: {:02X?})", addr, control_bytes);
            }
        }
    }

    #[cfg(feature = "ehal_0_2")]
    {
        let i2c = i2c as &mut dyn Write<Error = I2C::Error>;
        for addr in 0x03..=0x77 {
            if i2c.write(addr, control_bytes).is_ok() {
                log!(logger, "[ok] Found device at 0x{:02X} (ctrl bytes: {:02X?})", addr, control_bytes);
            }
        }
    }
    log!(logger, "[info] I2C scan complete.");
}

pub fn scan_init_sequence<I2C, L>(i2c: &mut I2C, logger: &mut L, init_sequence: &[u8])
where
    L: Logger,
    I2C: ?Sized,
{
    log!(logger, "[scan] Scanning I2C bus with init sequence: {:02X?}", init_sequence);

    let mut detected_cmds = Vec::<u8, 64>::new();

    for &cmd in init_sequence {
        log!(logger, "-> Testing command 0x{:02X}", cmd);

        #[cfg(feature = "ehal_1_0")]
        {
            let i2c = i2c as &mut dyn I2c<Error = I2C::Error>;
            for addr in 0x03..=0x77 {
                if i2c.write(addr, &[0x00, cmd]).is_ok() {
                    log!(logger, "[ok] Found device at 0x{:02X} responding to command 0x{:02X}", addr, cmd);
                }
            }
        }

        #[cfg(feature = "ehal_0_2")]
        {
            let i2c = i2c as &mut dyn Write<Error = I2C::Error>;
            for addr in 0x03..=0x77 {
                if i2c.write(addr, &[0x00, cmd]).is_ok() {
                    log!(logger, "[ok] Found device at 0x{:02X} responding to command 0x{:02X}", addr, cmd);
                }
            }
        }

        if detected_cmds.push(cmd).is_err() {
            log!(logger, "[warn] Detected commands buffer is full, results may be incomplete!");
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