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
                writeln!(writer, "Found").ok();
            }
            Err(e) => {
                let error_kind = e.to_compat(Some(addr));
                if error_kind == crate::error::ErrorKind::I2c(crate::error::I2cError::Nack) {
                    writeln!(writer, "No response (NACK)").ok();
                    continue;
                }
                write!(writer, "Write failed at ").ok();
                util::write_bytes_hex_fmt(writer, &[addr]).ok();
                writeln!(writer, ": {error_kind}").ok();
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
    writeln!(writer, "Scanning I2C bus with a single control byte...").ok();
    internal_scan(i2c, writer, &[ctrl_byte])
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
        format_args!("Starting I2C bus scan with initialization sequence..."),
    );
    write!(writer, "Initializing scan with control byte ").ok();
    util::write_bytes_hex_fmt(writer, &[ctrl_byte]).ok();
    writeln!(writer).ok();

    let found_addrs = match crate::scanner::scan_i2c(i2c, writer, ctrl_byte) {
        Ok(addrs) => addrs,
        Err(e) => {
            writeln!(writer, "Failed to scan I2C: {e:?}\r\n").ok();
            return Err(e);
        }
    };

    if found_addrs.is_empty() {
        writeln!(writer, "No I2C devices found.\r\n").ok();
        return Err(crate::error::ErrorKind::I2c(crate::error::I2cError::Nack));
    }

    let detected_cmds =
        check_init_sequence(i2c, writer, ctrl_byte, init_sequence, &found_addrs)?;

    let mut missing_cmds = heapless::Vec::<u8, INIT_SEQUENCE_LEN>::new();
    let mut sorted_detected_cmds = detected_cmds.clone();
    sorted_detected_cmds.sort_unstable();

    for &cmd in init_sequence.iter() {
        if sorted_detected_cmds.binary_search(&cmd).is_err() {
            missing_cmds.push(cmd).ok();
        }
    }

    write!(writer, "I2C Seq Scan Complete: Detected ").ok();
    for &cmd in detected_cmds.iter() {
        util::write_bytes_hex_fmt(writer, &[cmd]).ok();
        write!(writer, " ").ok();
    }
    write!(writer, " | Missing ").ok();
    for &cmd in missing_cmds.iter() {
        util::write_bytes_hex_fmt(writer, &[cmd]).ok();
        write!(writer, " ").ok();
    }
    writeln!(writer, "\r\n").ok();

    Ok(detected_cmds)
}

fn check_init_sequence<I2C, W, const N: usize>(
    i2c: &mut I2C,
    writer: &mut W,
    ctrl_byte: u8,
    init_sequence: &[u8; N],
    found_addrs: &heapless::Vec<u8, I2C_MAX_DEVICES>,
) -> Result<heapless::Vec<u8, N>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    W: core::fmt::Write,
{
    let mut detected_cmds = heapless::Vec::<u8, N>::new();
    let mut last_error: Option<crate::error::ErrorKind> = None;

    for &addr in found_addrs.iter() {
        write!(writer, "Testing init sequence on ").ok();
        util::write_bytes_hex_fmt(writer, &[addr]).ok();
        writeln!(writer, "...").ok();

        for &cmd in init_sequence.iter() {
            let mut command_data = heapless::Vec::<u8, 2>::new(); // ctrl_byte + cmd
            command_data.push(ctrl_byte).map_err(|_| {
                crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
            })?;
            command_data.push(cmd).map_err(|_| {
                crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
            })?;

            write!(writer, "  Sending command ").ok();
            util::write_bytes_hex_fmt(writer, &[cmd]).ok();
            write!(writer, " to ").ok();
            util::write_bytes_hex_fmt(writer, &[addr]).ok();
            writeln!(writer, "...").ok();

            match i2c.write(addr, &command_data) {
                Ok(_) => {
                    if !detected_cmds.contains(&cmd) {
                        detected_cmds.push(cmd).map_err(|_| {
                            crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
                        })?;
                    }
                    write!(writer, "  Command ").ok();
                    util::write_bytes_hex_fmt(writer, &[cmd]).ok();
                    writeln!(writer, " responded.").ok();
                }
                Err(e) => {
                    let error_kind = e.to_compat(Some(addr));
                    if error_kind == crate::error::ErrorKind::I2c(crate::error::I2cError::Nack) {
                        write!(writer, "  Command ").ok();
                        util::write_bytes_hex_fmt(writer, &[cmd]).ok();
                        writeln!(writer, " no response (NACK).").ok();
                        continue;
                    }
                    write!(writer, "  Write failed for ").ok();
                    util::write_bytes_hex_fmt(writer, &[cmd]).ok();
                    write!(writer, " at ").ok();
                    util::write_bytes_hex_fmt(writer, &[addr]).ok();
                    writeln!(writer, ": {error_kind}").ok();
                    last_error = Some(error_kind);
                }
            }
        }
    }

    if detected_cmds.is_empty() {
        Err(last_error.unwrap_or(crate::error::ErrorKind::I2c(crate::error::I2cError::Nack)))
    } else {
        Ok(detected_cmds)
    }
}
