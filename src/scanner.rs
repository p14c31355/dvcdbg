//! I2C scanner utilities
//!
//! Supports both embedded-hal 0.2 and 1.0 via cargo features:
//! - ehal_0_2
//! - ehal_1_0

use crate::log; // log! macro
use crate::logger::Logger;
#[cfg(feature = "ehal_0_2")]
use embedded_hal::blocking::i2c::Write as I2cWrite;
#[cfg(feature = "ehal_1_0")]
use embedded_hal_1::i2c::I2c;

#[cfg(feature = "ehal_0_2")]
fn write_0_2<I2C: I2cWrite>(i2c: &mut I2C, addr: u8, data: &[u8]) -> bool {
    i2c.write(addr, data).is_ok()
}

#[cfg(feature = "ehal_1_0")]
fn write_1_0<I2C: I2c>(i2c: &mut I2C, addr: u8, data: &[u8]) -> bool
where
    I2C::Error: core::fmt::Debug,
{
    I2c::write(i2c, addr, data).is_ok()
}

/// Internal scan logic shared between features
fn scan_logic<I2C, L>(
    i2c: &mut I2C,
    logger: &mut L,
    control_bytes: Option<&[u8]>,
    init_sequence: Option<&[u8]>,
    write_fn: &mut dyn FnMut(&mut I2C, u8, &[u8]) -> bool,
) where
    L: Logger,
{
    for addr in 0x03..=0x77 {
        let mut skip = false;

        if let Some(ctrl) = control_bytes {
            if !write_fn(i2c, addr, ctrl) {
                skip = true;
            }
        }

        if !skip {
            if let Some(init) = init_sequence {
                let _ = write_fn(i2c, addr, init);
            }
            log!(logger, "[ok] Found device at 0x{:02X}", addr);
        }
    }

    log!(logger, "[info] I2C scan complete.");
}

#[cfg(feature = "ehal_0_2")]
pub fn scan_i2c_inner_0_2<I2C, L>(
    i2c: &mut I2C,
    logger: &mut L,
    control_bytes: Option<&[u8]>,
    init_sequence: Option<&[u8]>,
) where
    I2C: I2cWrite,
    L: Logger,
{
    let mut write_fn = write_0_2::<I2C>;
    scan_logic(i2c, logger, control_bytes, init_sequence, &mut write_fn);
}

#[cfg(feature = "ehal_1_0")]
pub fn scan_i2c_inner_1_0<I2C, L>(
    i2c: &mut I2C,
    logger: &mut L,
    control_bytes: Option<&[u8]>,
    init_sequence: Option<&[u8]>,
) where
    I2C: I2c,
    I2C::Error: core::fmt::Debug,
    L: Logger,
{
    let mut write_fn = write_1_0::<I2C>;
    scan_logic(i2c, logger, control_bytes, init_sequence, &mut write_fn);
}

/// Public API
#[cfg(feature = "ehal_0_2")]
pub fn scan_i2c<I2C, L>(i2c: &mut I2C, logger: &mut L)
where
    I2C: I2cWrite,
    L: Logger,
{
    scan_i2c_inner_0_2(i2c, logger, None, None);
}

#[cfg(feature = "ehal_1_0")]
pub fn scan_i2c<I2C, L>(i2c: &mut I2C, logger: &mut L)
where
    I2C: I2c,
    I2C::Error: core::fmt::Debug,
    L: Logger,
{
    scan_i2c_inner_1_0(i2c, logger, None, None);
}

#[cfg(feature = "ehal_0_2")]
pub fn scan_i2c_with_ctrl<I2C, L>(i2c: &mut I2C, logger: &mut L, control_bytes: &[u8])
where
    I2C: I2cWrite,
    L: Logger,
{
    scan_i2c_inner_0_2(i2c, logger, Some(control_bytes), None);
}

#[cfg(feature = "ehal_1_0")]
pub fn scan_i2c_with_ctrl<I2C, L>(i2c: &mut I2C, logger: &mut L, control_bytes: &[u8])
where
    I2C: I2c,
    I2C::Error: core::fmt::Debug,
    L: Logger,
{
    scan_i2c_inner_1_0(i2c, logger, Some(control_bytes), None);
}

#[cfg(feature = "ehal_0_2")]
pub fn scan_init_sequence<I2C, L>(i2c: &mut I2C, logger: &mut L, init_sequence: &[u8])
where
    I2C: I2cWrite,
    L: Logger,
{
    scan_i2c_inner_0_2(i2c, logger, None, Some(init_sequence));
}

#[cfg(feature = "ehal_1_0")]
pub fn scan_init_sequence<I2C, L>(i2c: &mut I2C, logger: &mut L, init_sequence: &[u8])
where
    I2C: I2c,
    I2C::Error: core::fmt::Debug,
    L: Logger,
{
    scan_i2c_inner_1_0(i2c, logger, None, Some(init_sequence));
}
