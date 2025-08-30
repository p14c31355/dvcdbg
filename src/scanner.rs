// scanner.rs

//! Scanner utilities for I2C bus device discovery and analysis.
//!
//! This module provides functions to scan the I2C bus for connected devices,
//! optionally testing with control bytes or initialization command sequences.

use crate::compat::HalErrorExt;

pub const I2C_SCAN_ADDR_START: u8 = 0x03;
pub const I2C_SCAN_ADDR_END: u8 = 0x77;
pub const I2C_MAX_DEVICES: usize = 128;

/// Scans the I2C bus for devices that respond to a given data write.
///
/// It iterates through all possible I2C addresses and attempts to write the
/// provided `data`.
fn internal_scan<I2C, S>(
    i2c: &mut I2C,
    serial: &mut S,
    data: &[u8],
    log_level: crate::explore::logger::LogLevel,
) -> Result<heapless::Vec<u8, I2C_MAX_DEVICES>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let mut found_addrs = heapless::Vec::<u8, I2C_MAX_DEVICES>::new();
    let mut last_error: Option<crate::error::ErrorKind> = None;

    for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
        if let crate::explore::logger::LogLevel::Verbose = log_level {
            write!(serial, "[LOG] Scanning 0x{:02x}...", addr).ok();
        }

        match i2c.write(addr, data) {
            Ok(_) => {
                found_addrs.push(addr).map_err(|_| {
                    crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
                })?;
                if let crate::explore::logger::LogLevel::Verbose = log_level {
                    writeln!(serial, " Found").ok();
                }
            }
            Err(e) => {
                let error_kind = e.to_compat(Some(addr));
                if error_kind == crate::error::ErrorKind::I2c(crate::error::I2cError::Nack) {
                    if let crate::explore::logger::LogLevel::Verbose = log_level {
                        writeln!(serial, " No response (NACK)").ok();
                    }
                    continue;
                }
                if let crate::explore::logger::LogLevel::Verbose = log_level {
                    writeln!(
                        serial,
                        "[ERROR] Write failed at 0x{:02x}: {}",
                        addr, error_kind
                    )
                    .ok();
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
pub fn scan_i2c<I2C, S>(
    i2c: &mut I2C,
    serial: &mut S,
    ctrl_byte: u8,
    log_level: crate::explore::logger::LogLevel,
) -> Result<heapless::Vec<u8, I2C_MAX_DEVICES>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    if let crate::explore::logger::LogLevel::Verbose = log_level {
        writeln!(
            serial,
            "[LOG] Scanning I2C bus with a single control byte..."
        )
        .ok();
    }
    internal_scan(i2c, serial, &[ctrl_byte], log_level)
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
pub fn scan_init_sequence<I2C, S, const N: usize>(
    i2c: &mut I2C,
    serial: &mut S,
    ctrl_byte: u8,
    init_sequence: &[u8; N],
    log_level: crate::explore::logger::LogLevel,
) -> Result<heapless::Vec<u8, N>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    if let crate::explore::logger::LogLevel::Verbose = log_level {
        writeln!(
            serial,
            "[LOG] Starting I2C bus scan with initialization sequence..."
        )
        .ok();
        writeln!(
            serial,
            "[INFO] Initializing scan with control byte 0x{:02x}",
            ctrl_byte
        )
        .ok();
    }

    let initial_found_addrs = internal_scan(i2c, serial, &[ctrl_byte], log_level)?;
    let mut detected_cmds = check_init_sequence(
        i2c,
        serial,
        ctrl_byte,
        init_sequence,
        log_level,
        &initial_found_addrs,
    )?;

    if let crate::explore::logger::LogLevel::Verbose = log_level {
        writeln!(serial, "[INFO] I2C scan with init sequence complete.").ok();
    }
    log_sequence_summary(serial, init_sequence, &mut detected_cmds); // detected_cmdsを可変参照に変更
    Ok(detected_cmds)
}

/// Checks which commands in the init sequence are responded to by the initially found devices.
fn check_init_sequence<I2C, S, const N: usize>(
    i2c: &mut I2C,
    serial: &mut S,
    ctrl_byte: u8,
    init_sequence: &[u8; N],
    log_level: crate::explore::logger::LogLevel,
    initial_found_addrs: &heapless::Vec<u8, I2C_MAX_DEVICES>,
) -> Result<heapless::Vec<u8, N>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let mut detected_cmds = heapless::Vec::<u8, N>::new();
    let mut last_error: Option<crate::error::ErrorKind> = None;

    for &seq_cmd in init_sequence.iter() {
        match internal_scan(i2c, serial, &[ctrl_byte, seq_cmd], log_level) {
            Ok(responded_addrs) => {
                if responded_addrs
                    .iter()
                    .any(|addr| initial_found_addrs.contains(addr))
                {
                    if let Err(_) = detected_cmds.push(seq_cmd) {
                        writeln!(serial, "[WARN] Detected commands buffer overflow. Some commands may be truncated.").ok();
                        break;
                    }
                }
            }
            Err(e) => {
                if let crate::explore::logger::LogLevel::Verbose = log_level {
                    writeln!(
                        serial,
                        "[ERROR] Scan failed for command 0x{:02x}: {:?}",
                        seq_cmd, e
                    )
                    .ok();
                }
                last_error = Some(e);
            }
        }
    }

    if detected_cmds.is_empty() {
        Err(last_error.unwrap_or(crate::error::ErrorKind::I2c(crate::error::I2cError::Nack)))
    } else {
        Ok(detected_cmds)
    }
}

/// Logs a summary of the expected and detected commands from an initialization sequence scan.
fn log_sequence_summary<W: core::fmt::Write, const N: usize>(
    serial: &mut W,
    expected_sequence: &[u8; N],
    detected_cmds: &mut heapless::Vec<u8, N>,
) {
    let mut missing_cmds = heapless::Vec::<u8, N>::new();

    detected_cmds.sort_unstable();

    for &cmd in expected_sequence.iter() {
        if detected_cmds.binary_search(&cmd).is_err() {
            if missing_cmds.push(cmd).is_err() {
                writeln!(
                    serial,
                    "[WARN] Missing commands buffer is full, list is truncated."
                )
                .ok();
                break;
            }
        }
    }

    writeln!(serial, "\n--- I2C Sequence Scan Summary ---").ok();

    writeln!(serial, "Expected Commands:").ok();
    for cmd_ref in expected_sequence.iter() {
        write!(serial, " 0x{:02x}", *cmd_ref).ok();
    }
    writeln!(serial, "\n").ok();

    writeln!(serial, "Commands That Responded:").ok();
    for cmd_ref in detected_cmds.iter() {
        write!(serial, " 0x{:02x}", *cmd_ref).ok();
    }
    writeln!(serial, "\n").ok();

    writeln!(serial, "Commands With No Response:").ok();
    for cmd_ref in missing_cmds.iter() {
        write!(serial, " 0x{:02x}", *cmd_ref).ok();
    }
    writeln!(serial, "\n--- End Summary ---").ok();
}
