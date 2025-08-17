/// Scanner utilities for I2C bus device discovery and analysis.
///
/// Supports both embedded-hal 0.2.x and 1.0.x through feature flags:
/// - `ehal_0_2` → uses `blocking::i2c::Write`
/// - `ehal_1_0` → uses `i2c::I2c`
use crate::logger::Logger;

#[cfg(feature = "ehal_1_0")]
use embedded_hal_1::i2c::I2c;

#[cfg(feature = "ehal_0_2")]
use embedded_hal_0_2::blocking::i2c::Write as I2cWrite;

/// 内部共通関数
fn scan_i2c_inner<I2C, L>(
    i2c: &mut I2C,
    logger: &mut L,
    control_bytes: Option<&[u8]>,
    init_sequence: Option<&[u8]>,
) where
    L: Logger,
    #[cfg(feature = "ehal_0_2")] I2C: I2cWrite,
    #[cfg(feature = "ehal_1_0")] I2C: I2c,
    #[cfg(feature = "ehal_1_0")] I2C::Error: core::fmt::Debug,
{
    #[cfg(feature = "ehal_0_2")]
    let mut write_fn = |i2c: &mut I2C, addr: u8, data: &[u8]| -> bool {
        i2c.write(addr, data).is_ok()
    };

    #[cfg(feature = "ehal_1_0")]
    let mut write_fn = |i2c: &mut I2C, addr: u8, data: &[u8]| -> bool {
        i2c.write(addr, data).is_ok()
    };

    if let Some(seq) = init_sequence {
        for &cmd in seq {
            for addr in 0x03..=0x77 {
                if write_fn(i2c, addr, &[0x00, cmd]) {
                    log!(logger, "[ok] Found device at 0x{:02X} responding to 0x{:02X}", addr, cmd);
                }
            }
        }
        log!(logger, "[info] I2C scan with init sequence complete.");
        return;
    }

    let ctrl = control_bytes.unwrap_or(&[]);
    for addr in 0x03..=0x77 {
        if write_fn(i2c, addr, ctrl) {
            log!(logger, "[ok] Found device at 0x{:02X}", addr);
        }
    }
    log!(logger, "[info] I2C scan complete.");
}

/// パブリック API
pub fn scan_i2c<I2C, L>(i2c: &mut I2C, logger: &mut L)
where
    L: Logger,
    #[cfg(feature = "ehal_0_2")] I2C: I2cWrite,
    #[cfg(feature = "ehal_1_0")] I2C: I2c,
    #[cfg(feature = "ehal_1_0")] I2C::Error: core::fmt::Debug,
{
    scan_i2c_inner(i2c, logger, None, None);
}

pub fn scan_i2c_with_ctrl<I2C, L>(i2c: &mut I2C, logger: &mut L, control_bytes: &[u8])
where
    L: Logger,
    #[cfg(feature = "ehal_0_2")] I2C: I2cWrite,
    #[cfg(feature = "ehal_1_0")] I2C: I2c,
    #[cfg(feature = "ehal_1_0")] I2C::Error: core::fmt::Debug,
{
    scan_i2c_inner(i2c, logger, Some(control_bytes), None);
}

pub fn scan_init_sequence<I2C, L>(i2c: &mut I2C, logger: &mut L, init_sequence: &[u8])
where
    L: Logger,
    #[cfg(feature = "ehal_0_2")] I2C: I2cWrite,
    #[cfg(feature = "ehal_1_0")] I2C: I2c,
    #[cfg(feature = "ehal_1_0")] I2C::Error: core::fmt::Debug,
{
    scan_i2c_inner(i2c, logger, None, Some(init_sequence));
}