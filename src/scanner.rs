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
        write!(writer, "Scanning ").ok();
        util::write_bytes_hex_fmt(writer, &[addr]).ok();
        writeln!(writer, "...").ok();

        match i2c.write(addr, data) {
            Ok(_) => {
                found_addrs.push(addr).map_err(|_| {
                    crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
                })?;
                writeln!(writer, " OK.").ok();
            }
            Err(e) => {
                let error_kind = e.to_compat(Some(addr));
                last_error = Some(error_kind);
                writeln!(writer, " FAILED: {error_kind:?}").ok();
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
        writeln!(writer, "Scanning for init command {i}: {cmd:02X?}").ok();
        let cmd_slice = [cmd];
        match internal_scan(i2c, writer, &cmd_slice) {
            Ok(found_addrs) => {
                writeln!(writer, " -> Found on addresses: {found_addrs:02X?}").ok();
                detected_cmds.push(cmd).ok();
            }
            Err(error_kind) => {
                writeln!(writer, " -> Not found. Error: {error_kind:?}").ok();
                last_error = Some(error_kind);
            }
        }
    }

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
    let mut missing_cmds = heapless::Vec::<u8, N>::new();

    writeln!(writer, "\n--- I2C Sequence Scan Summary ---").ok();

    // Log detected commands in their original, unsorted order
    write!(writer, "Commands That Responded:").ok();
    for &cmd in detected_cmds.iter() {
        write!(writer, " ").ok();
        util::write_bytes_hex_fmt(writer, &[cmd]).ok();
    }
    writeln!(writer).ok();

    // Sort in-place to find missing commands efficiently
    detected_cmds.sort_unstable();

    for &cmd in expected_sequence.iter() {
        if detected_cmds.binary_search(&cmd).is_err() {
            missing_cmds.push(cmd).ok();
        }
    }

    write!(writer, "Expected Commands:").ok();
    for &cmd in expected_sequence {
        write!(writer, " ").ok();
        util::write_bytes_hex_fmt(writer, &[cmd]).ok();
    }
    writeln!(writer).ok();

    write!(writer, "Commands Not Found:").ok();
    if missing_cmds.is_empty() {
        writeln!(writer, " (None)").ok();
    } else {
        for &cmd in missing_cmds.iter() {
            write!(writer, " ").ok();
            util::write_bytes_hex_fmt(writer, &[cmd]).ok();
        }
        writeln!(writer).ok();
    }
}
