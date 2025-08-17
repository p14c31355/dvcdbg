use crate::log;
use crate::logger::Logger;
#[cfg(feature = "ehal_0_2")]
use embedded_hal::blocking::i2c::Write as I2cWrite;
#[cfg(feature = "ehal_1_0")]
use embedded_hal_1::i2c::I2c;

/// Inner scan logic (0.2)
#[cfg(feature = "ehal_0_2")]
fn scan_i2c_inner_0_2<I2C, L>(
    i2c: &mut I2C,
    logger: &mut L,
    control_bytes: Option<&[u8]>,
    init_sequence: Option<&[u8]>,
) where
    I2C: I2cWrite,
    L: Logger,
{
    // Actual I2C writing and logging processing
    if let Some(ctrl) = control_bytes {
        let _ = i2c.write(0x00, ctrl); // Temporary address and writing
    }
    if let Some(seq) = init_sequence {
        let _ = i2c.write(0x00, seq);
    }
    log!(logger, "[info] I2C scan complete.");
}

/// Inner scan logic (1.0)
#[cfg(feature = "ehal_1_0")]
fn scan_i2c_inner_1_0<I2C, L>(
    i2c: &mut I2C,
    logger: &mut L,
    control_bytes: Option<&[u8]>,
    init_sequence: Option<&[u8]>,
) where
    I2C: I2c,
    I2C::Error: core::fmt::Debug,
    L: Logger,
{
    if let Some(ctrl) = control_bytes {
        I2C::write(i2c, 0x00, ctrl).unwrap();
    }
    if let Some(seq) = init_sequence {
        I2C::write(i2c, 0x00, seq).unwrap();
    }
    log!(logger, "[info] I2C scan complete.");
}

// ------------------- Public API -------------------

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
