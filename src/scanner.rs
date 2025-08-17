/// Scanner utilities for I2C bus device discovery and analysis.
///
/// Supports both embedded-hal 0.2.x and 1.0.x through feature flags:
/// - `ehal_0_2` → uses `blocking::i2c::Write`
/// - `ehal_1_0` → uses `i2c::I2c`
use crate::log;
use crate::logger::Logger;
use heapless::Vec;

#[cfg(feature = "ehal_1_0")]
use embedded_hal_1::i2c::I2c;

#[cfg(feature = "ehal_0_2")]
use embedded_hal_0_2::blocking::i2c::Write as I2cWrite;

/// 内部共通関数にクロージャを使う方式
fn scan_i2c_inner<I2C, L>(
    i2c: &mut I2C,
    logger: &mut L,
    control_bytes: Option<&[u8]>,
    init_sequence: Option<&[u8]>,
) where
    L: Logger,
{
    #[cfg(feature = "ehal_0_2")]
    let mut write_fn = |i2c: &mut I2C, addr: u8, data: &[u8]| -> bool {
        i2c.write(addr, data).is_ok()
    };

    #[cfg(feature = "ehal_1_0")]
    let mut write_fn = |i2c: &mut I2C, addr: u8, data: &[u8]| -> bool
    where
        I2C: I2c,
        I2C::Error: core::fmt::Debug,
    {
        i2c.write(addr, data).is_ok()
    };

    if let Some(seq) = init_sequence {
        log!(logger, "[scan] Scanning I2C with init sequence {:02X?}", seq);
        let mut detected_cmds = Vec::<u8, 64>::new();

        for &cmd in seq {
            log!(logger, "-> Testing command 0x{:02X}", cmd);

            for addr in 0x03..=0x77 {
                if write_fn(i2c, addr, &[0x00, cmd]) {
                    log!(logger, "[ok] Found device at 0x{:02X} responding to 0x{:02X}", addr, cmd);
                }
            }

            if detected_cmds.push(cmd).is_err() {
                log!(logger, "[warn] Detected commands buffer full!");
            }
        }

        detected_cmds.sort_unstable();
        let missing_cmds: Vec<u8, 64> = seq
            .iter()
            .filter(|&&c| detected_cmds.binary_search(&c).is_err())
            .copied()
            .collect();

        log!(logger, "[info] Expected sequence: {:02X?}", seq);
        log!(logger, "[info] Commands with response: {:02X?}", detected_cmds.as_slice());
        log!(logger, "[info] Commands with no response: {:02X?}", missing_cmds.as_slice());
        log!(logger, "[info] I2C scan with init sequence complete.");
        return;
    }

    let ctrl = control_bytes.unwrap_or(&[]);
    log!(logger, "[scan] Scanning I2C bus{}", if ctrl.is_empty() { "" } else { " with control bytes" });

    for addr in 0x03..=0x77 {
        if write_fn(i2c, addr, ctrl) {
            if ctrl.is_empty() {
                log!(logger, "[ok] Found device at 0x{:02X}", addr);
            } else {
                log!(logger, "[ok] Found device at 0x{:02X} (ctrl bytes: {:02X?})", addr, ctrl);
            }
        }
    }

    log!(logger, "[info] I2C scan complete.");
}

/// Public API
pub fn scan_i2c<I2C, L>(i2c: &mut I2C, logger: &mut L)
where
    L: Logger,
{
    scan_i2c_inner(i2c, logger, None, None);
}

pub fn scan_i2c_with_ctrl<I2C, L>(i2c: &mut I2C, logger: &mut L, control_bytes: &[u8])
where
    L: Logger,
{
    scan_i2c_inner(i2c, logger, Some(control_bytes), None);
}

pub fn scan_init_sequence<I2C, L>(i2c: &mut I2C, logger: &mut L, init_sequence: &[u8])
where
    L: Logger,
{
    scan_i2c_inner(i2c, logger, None, Some(init_sequence));
}
