//! Scanner utilities for I2C bus device discovery and analysis.

use crate::compat::HalErrorExt;
use crate::compat::util;

pub const I2C_SCAN_ADDR_START: u8 = 0x03;
pub const I2C_SCAN_ADDR_END: u8 = 0x77;
pub const I2C_MAX_DEVICES: usize = 128;

/// Scans the I2C bus for devices that respond to a given data write.
///
/// It iterates through all possible I2C addresses and attempts to write the
/// provided `data`.
fn internal_scan<I2C, W>(
    i2c: &mut I2C,
    writer: &mut W,
    data: &[u8],
) -> Result<heapless::Vec<u8, I2C_MAX_DEVICES>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    W: core::fmt::Write,
{
    let mut found_addrs = heapless::Vec::<u8, I2C_MAX_DEVICES>::new();
    let mut last_error: Option<crate::error::ErrorKind> = None;

    for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
        util::prevent_garbled(
            writer,
            format_args!("Scanning {:02X?}...", addr),
        );

        match i2c.write(addr, data) {
            Ok(_) => {
                found_addrs.push(addr).map_err(|_| {
                    crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
                })?;
                util::prevent_garbled(writer, format_args!(" OK."));
            }
            Err(e) => {
                let error_kind = e.to_compat(Some(addr));
                last_error = Some(error_kind);
                util::prevent_garbled(writer, format_args!(" FAILED: {error_kind:?}"));
            }
        }
    }

    if found_addrs.is_empty() {
        Err(last_error.unwrap_or(crate::error::ErrorKind::I2c(crate::error::I2cError::Nack)))
    } else {
        Ok(found_addrs)
    }
}

/// Scans the I2C bus for devices by attempting to write a single control byte to each address.
///
/// # Parameters
///
/// - `i2c`: The I2C bus instance.
/// - `serial`: The serial writer for logging.
/// - `ctrl_byte`: The control byte.
/// - `log_level`: The desired logging level.
pub fn scan_i2c<I2C, W>(
    i2c: &mut I2C,
    writer: &mut W,
    prefix: u8,
) -> Result<heapless::Vec<u8, I2C_MAX_DEVICES>, crate::error::ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    W: core::fmt::Write,
{
    let prefix_slice = [prefix];
    internal_scan(i2c, writer, &prefix_slice).map_err(|e| e.into())
}

pub fn scan_init_sequence<I2C, W, const N: usize>(
    i2c: &mut I2C,
    writer: &mut W,
    init_sequence: &[u8; N],
) -> Result<heapless::Vec<u8, N>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    W: core::fmt::Write,
{
    let mut detected_cmds = heapless::Vec::<u8, N>::new();
    let mut last_error: Option<crate::error::ErrorKind> = None;

    for (i, &cmd) in init_sequence.iter().enumerate() {
        util::prevent_garbled(
            writer,
            format_args!("Scanning for init command {i}: {cmd:02X?}"),
        );
        let cmd_slice = [cmd];
        match internal_scan(i2c, writer, &cmd_slice) {
            Ok(found_addrs) => {
                util::prevent_garbled(
                    writer,
                    format_args!(" -> Found on addresses: {found_addrs:02X?}"),
                );
                detected_cmds.push(cmd).ok();
            }
            Err(error_kind) => {
                util::prevent_garbled(
                    writer,
                    format_args!(" -> Not found. Error: {error_kind:?}"),
                );
                last_error = Some(error_kind);
            }
        }
    }
    
    log_sequence_summary(writer, init_sequence, &mut detected_cmds);

    if detected_cmds.is_empty() {
        Err(last_error.unwrap_or(crate::error::ErrorKind::I2c(crate::error::I2cError::Nack)))
    } else {
        Ok(detected_cmds)
    }
}

fn log_sequence_summary<W, const N: usize>(
    writer: &mut W,
    expected_sequence: &[u8; N],
    detected_cmds: &mut heapless::Vec<u8, N>,
) where
    W: core::fmt::Write,
{
    util::prevent_garbled(writer, format_args!("\n--- I2C Sequence Scan Summary ---"));

    util::prevent_garbled(writer, format_args!("Commands That Responded:"));
    for &cmd in detected_cmds.iter() {
        util::prevent_garbled(writer, format_args!(" {:02X}", cmd));
    }
    util::prevent_garbled(writer, format_args!(""));

    detected_cmds.sort_unstable();

    util::prevent_garbled(writer, format_args!("Expected Commands:"));
    for &cmd in expected_sequence {
        util::prevent_garbled(writer, format_args!(" {:02X}", cmd));
    }
    util::prevent_garbled(writer, format_args!(""));

    util::prevent_garbled(writer, format_args!("Commands Not Found:"));
    let mut found_missing = false;
    for &cmd in expected_sequence.iter() {
        if detected_cmds.binary_search(&cmd).is_err() {
            util::prevent_garbled(writer, format_args!(" {:02X}", cmd));
            found_missing = true;
        }
    }

    if !found_missing {
        util::prevent_garbled(writer, format_args!(" (None)"));
    }
    util::prevent_garbled(writer, format_args!(""));
}