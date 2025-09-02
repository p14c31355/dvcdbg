//! Scanner utilities for I2C bus device discovery and analysis.

use crate::compat::HalErrorExt;
use crate::compat::util;
use core::fmt::Write;
use heapless::String;

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
        match i2c.write(addr, data) {
            Ok(_) => {
                if found_addrs.push(addr).is_err() {
                    return Err(crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow));
                }
            }
            Err(e) => {
                let error_kind = e.to_compat(Some(addr));
                if error_kind == crate::error::ErrorKind::I2c(crate::error::I2cError::Nack) {
                    continue;
                }
                last_error = Some(error_kind);
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
    ctrl_byte: u8,
) -> Result<heapless::Vec<u8, I2C_MAX_DEVICES>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    W: core::fmt::Write,
{
    util::prevent_garbled(
        writer,
        format_args!("Scanning I2C bus with a {ctrl_byte:02X} ..."),
    );
    let found_addrs = internal_scan(i2c, writer, &[ctrl_byte])?;
    util::prevent_garbled(writer, format_args!("Found device @ {:02X}", found_addrs[0]));
    Ok(found_addrs)
}

/// Scans the I2C bus for devices that respond to a given initialization sequence.
///
/// This function first performs an initial scan to find all responding devices,
/// then iterates through the `init_sequence` to find which commands elicit a response
/// from those found devices.
///
/// # Parameters
///
/// - `i2c`: The I2C bus instance.
/// - `serial`: The serial writer for logging.
/// - `ctrl_byte`: The control byte to be sent before each command in the sequence.
/// - `init_sequence`: The sequence of bytes to test.
/// - `log_level`: The desired logging level.
///
/// # Returns
///
/// A `heapless::Vec<u8, N>` containing the bytes from `init_sequence` that elicited a response.
pub fn scan_init_sequence<I2C, W, const INIT_SEQUENCE_LEN: usize>(
    i2c: &mut I2C,
    writer: &mut W,
    ctrl_byte: u8,
    init_sequence: &[u8; INIT_SEQUENCE_LEN],
) -> Result<heapless::Vec<u8, INIT_SEQUENCE_LEN>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    W: core::fmt::Write,
{
    util::prevent_garbled(
        writer,
        format_args!("Start I2C scan with INIT_SEQ..."),
    );
    util::prevent_garbled(
        writer,
        format_args!("Initializing scan with ctrl byte {ctrl_byte:02X}"),
    );

    let found_addrs = match crate::scanner::scan_i2c(i2c, writer, ctrl_byte) {
        Ok(addrs) => addrs,
        Err(e) => {
            util::prevent_garbled(
                writer,
                format_args!("Failed to scan I2C: {e:?}"),
            );
            return Err(e);
        }
    };

    if found_addrs.is_empty() {
        util::prevent_garbled(writer, format_args!("No devices found."));
        return Err(crate::error::ErrorKind::I2c(crate::error::I2cError::Nack));
    }

    let mut detected_cmds = heapless::Vec::<u8, INIT_SEQUENCE_LEN>::new();
    let mut last_error: Option<crate::error::ErrorKind> = None;

    for &addr in found_addrs.iter() {
        util::prevent_garbled(
            writer,
            format_args!("Testing init SEQ on {addr:02X}..."),
        );

        for &cmd in init_sequence.iter() {
            let mut command_data = heapless::Vec::<u8, 3>::new(); // ctrl_byte + cmd
            command_data.push(ctrl_byte).map_err(|_| {
                crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
            })?;
            command_data.push(cmd).map_err(|_| {
                crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
            })?;

            util::prevent_garbled(
                writer,
                format_args!("  Sending command {cmd:02X} to {addr:02X}..."),
            );

            match i2c.write(addr, &command_data) {
                Ok(_) => {
                    if !detected_cmds.contains(&cmd) {
                        detected_cmds.push(cmd).map_err(|_| {
                            crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
                        })?;
                    }
                    util::prevent_garbled(
                        writer,
                        format_args!("  Command {cmd:02X} responded."),
                    );
                }
                Err(e) => {
                    let error_kind = e.to_compat(Some(addr));
                    if error_kind == crate::error::ErrorKind::I2c(crate::error::I2cError::Nack) {
                        util::prevent_garbled(
                            writer,
                            format_args!("  Command {cmd:02X} no response (NACK)."),
                        );
                        continue;
                    }
                    util::prevent_garbled(
                        writer,
                        format_args!("  Write failed for {cmd:02X} at {addr:02X}: {error_kind:?}"),
                    );
                    last_error = Some(error_kind);
                }
            }
        }
    }

    let mut missing_cmds = heapless::Vec::<u8, INIT_SEQUENCE_LEN>::new();
    let mut sorted_detected = detected_cmds.clone();
    sorted_detected.sort_unstable();
    for &b in init_sequence {
        if sorted_detected.binary_search(&b).is_err() {
            if missing_cmds.push(b).is_err() {
                writeln!(
                    writer,
                    "[W] Missing commands buffer full, list truncated."
                )
                .ok();
                break;
            }
        }
    }

    writeln!(writer, "Expected sequence:").ok();
    for b in init_sequence {
        write!(writer, " ").ok();
        crate::compat::util::write_bytes_hex_fmt(writer, &[*b]).ok();
        writeln!(writer).ok();
    }
    writeln!(writer).ok();

    writeln!(writer, "Commands with response:").ok();
    let mut detected_cmds_for_log = heapless::Vec::<u8, 64>::new();
    for &cmd in detected_cmds.iter() {
        if detected_cmds_for_log.push(cmd).is_err() {
            // If the buffer overflows, truncate the list for logging
            break;
        }
    }
    for b in &detected_cmds_for_log {
        write!(writer, " ").ok();
        crate::compat::util::write_bytes_hex_fmt(writer, &[*b]).ok();
        writeln!(writer).ok();
    }
    writeln!(writer).ok();

    writeln!(writer, "Commands with no response:").ok();
    for b in &missing_cmds {
        write!(writer, " ").ok();
        crate::compat::util::write_bytes_hex_fmt(writer, &[*b]).ok();
        writeln!(writer).ok();
    }
    writeln!(writer).ok();

    if detected_cmds.is_empty() {
        Err(last_error.unwrap_or(crate::error::ErrorKind::I2c(crate::error::I2cError::Nack)))
    } else {
        Ok(detected_cmds)
    }
}
