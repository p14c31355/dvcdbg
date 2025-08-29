// scanner.rs
//! Scanner utilities for I2C bus device discovery and analysis.
//!
//! This module provides functions to scan the I2C bus for connected devices,
//! optionally testing with control bytes or initialization command sequences.

use crate::compat::{HalErrorExt, ascii};
use core::fmt::Write;
use heapless::Vec;

pub const I2C_SCAN_ADDR_START: u8 = 0x03;
pub const I2C_SCAN_ADDR_END: u8 = 0x77;

pub const I2C_MAX_DEVICES: usize = 128;
pub const I2C_BUFFER_SIZE: usize = 512;

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
            write!(serial, "[log] Scanning 0x").ok();
            crate::compat::ascii::write_bytes_hex_fmt(serial, &[addr]).ok();
            write!(serial, "...").ok();
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
                let e_kind = e.to_compat(Some(addr));
                if e_kind == crate::error::ErrorKind::I2c(crate::error::I2cError::Nack) {
                    if let crate::explore::logger::LogLevel::Verbose = log_level {
                        writeln!(serial, " No response (NACK)").ok();
                    }
                    continue;
                }
                if let crate::explore::logger::LogLevel::Verbose = log_level {
                    write!(serial, "[error] write failed at ").ok();
                    crate::compat::ascii::write_bytes_hex_fmt(serial, &[addr]).ok();
                    writeln!(serial, ": {}", e_kind).ok();
                }
                last_error = Some(e_kind);
            }
        }
    }
    if found_addrs.is_empty() {
        Err(last_error.unwrap_or(crate::error::ErrorKind::I2c(crate::error::I2cError::Nack)))
    } else {
        Ok(found_addrs)
    }
}

/// Macro to define common I2C scanner functions.
///
/// This macro generates `scan_i2c`, `scan_i2c_with_ctrl`, and `scan_init_sequence`
/// functions, which are used to discover I2C devices.
///
/// # Parameters
///
/// - `$i2c_trait`: The trait that defines the I2C interface (e.g., `embedded_hal::i2c::I2c`).
/// - `$error_trait`: The trait that defines the I2C error type (e.g., `embedded_hal::i2c::Error`).
/// - `$write_trait`: The trait that defines the serial writer (e.g., `core::fmt::Write`).
macro_rules! define_scanner {
    ($i2c_trait:path, $error_trait:path, $write_trait:path) => {
        /// Scans the I2C bus for devices by attempting to write a single byte (0x00) to each address.
        ///
        /// # Parameters
        ///
        /// - `i2c`: The I2C bus instance.
        /// - `serial`: The serial writer for logging.
        /// - `ctrl_byte`: The control byte
        /// - `log_level`: The desired logging level.
        pub fn scan_i2c<I2C, S>(
            i2c: &mut I2C,
            serial: &mut S,
            ctrl_byte: &[u8],
            log_level: $crate::explore::logger::LogLevel,
        ) -> Result<heapless::Vec<u8, I2C_MAX_DEVICES>, $crate::error::ErrorKind>
        where
            I2C: $i2c_trait,
            <I2C as $i2c_trait>::Error: $crate::compat::HalErrorExt,
            S: $write_trait,
        {
            if let $crate::explore::logger::LogLevel::Verbose = log_level {
                writeln!(serial, "[log] Scanning I2C bus...").ok();
            }
            let result = $crate::scanner::internal_scan(i2c, serial, ctrl_byte, log_level);
            result
        }

        /// Scans the I2C bus for devices that respond to a given initialization sequence.
        ///
        /// This function iterates through each byte in `init_sequence` and attempts to write it
        /// to all I2C addresses. It returns a `Vec` of the bytes from `init_sequence` that
        /// received a response from at least one device.
        ///
        /// # Parameters
        ///
        /// - `i2c`: The I2C bus instance.
        /// - `serial`: The serial writer for logging.
        /// - `init_sequence`: The sequence of bytes to test.
        /// - `log_level`: The desired logging level.
        ///
        /// # Returns
        ///
        /// A `heapless::Vec<u8, 64>` containing the bytes from `init_sequence` that elicited a response.
        pub fn scan_init_sequence<I2C, S>(
            i2c: &mut I2C,
            serial: &mut S,
            ctrl_byte: u8,
            init_sequence: &[u8],
            log_level: $crate::explore::logger::LogLevel,
        ) -> Result<heapless::Vec<u8, I2C_BUFFER_SIZE>, $crate::error::ErrorKind>
        where
            I2C: $i2c_trait,
            <I2C as $i2c_trait>::Error: $crate::compat::HalErrorExt,
            S: $write_trait,
        {
            if let $crate::explore::logger::LogLevel::Verbose = log_level {
                writeln!(serial, "[scan] Scanning I2C bus with init sequence:").ok();
                for chunk in init_sequence.chunks(16) {
                    write!(serial, " ").ok();
                    $crate::compat::ascii::write_bytes_hex_fmt(serial, chunk).ok();
                    writeln!(serial).ok();
                }
            }
            let initial_found_addrs =
                $crate::scanner::scan_i2c(i2c, serial, &[ctrl_byte], log_level)?;

            // Call the extracted helper function
            let detected_cmds = $crate::scanner::sequence_iterative_check(
                i2c,
                serial,
                ctrl_byte,
                init_sequence,
                log_level,
                &initial_found_addrs,
            )?;

            if let $crate::explore::logger::LogLevel::Verbose = log_level {
                writeln!(serial, "[info] I2C scan with init sequence complete.").ok();
            }
            $crate::scanner::log_differences(serial, init_sequence, &detected_cmds);
            Ok(detected_cmds)
        }

        fn log_differences<W: core::fmt::Write>(
            serial: &mut W,
            expected: &[u8],
            detected: &heapless::Vec<u8, I2C_BUFFER_SIZE>,
        ) {
            let mut missing_cmds = heapless::Vec::<u8, I2C_BUFFER_SIZE>::new();
            let mut sorted_detected = detected.clone();
            sorted_detected.sort_unstable();
            for &b in expected {
                if sorted_detected.binary_search(&b).is_err() {
                    if missing_cmds.push(b).is_err() {
                        writeln!(
                            serial,
                            "[warn] Missing commands buffer is full, list is truncated."
                        )
                        .ok();
                        break;
                    }
                }
            }

            writeln!(serial, "Expected sequence:").ok();
            for b in expected {
                write!(serial, " ").ok();
                ascii::write_bytes_hex_fmt(serial, &[*b]).ok();
                writeln!(serial).ok();
            }
            writeln!(serial).ok();

            writeln!(serial, "Commands with response:").ok();
            for b in detected {
                write!(serial, " ").ok();
                ascii::write_bytes_hex_fmt(serial, &[*b]).ok();
                writeln!(serial).ok();
            }
            writeln!(serial).ok();

            writeln!(serial, "Commands with no response:").ok();
            for b in &missing_cmds {
                write!(serial, " ").ok();
                ascii::write_bytes_hex_fmt(serial, &[*b]).ok();
                writeln!(serial).ok();
            }
            writeln!(serial).ok();
        }
    };
}

fn sequence_iterative_check<I2C, S>(
    i2c: &mut I2C,
    serial: &mut S,
    ctrl_byte: u8,
    init_sequence: &[u8],
    log_level: crate::explore::logger::LogLevel,
    initial_found_addrs: &heapless::Vec<u8, I2C_MAX_DEVICES>,
) -> Result<heapless::Vec<u8, I2C_BUFFER_SIZE>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let mut detected_cmds = heapless::Vec::<u8, I2C_BUFFER_SIZE>::new();
    let mut last_error: Option<crate::error::ErrorKind> = None;

    for &seq_cmd in init_sequence.iter() {
        match crate::scanner::internal_scan(i2c, serial, &[ctrl_byte, seq_cmd], log_level) {
            Ok(responded_addrs_for_cmd) => {
                let mut cmd_responded_by_initial_device = false;
                for &addr in responded_addrs_for_cmd.iter() {
                    if initial_found_addrs.contains(&addr) {
                        if let crate::explore::logger::LogLevel::Verbose = log_level {
                            write!(serial, "[ok] Found device at ").ok();
                            crate::compat::ascii::write_bytes_hex_fmt(serial, &[addr]).ok();
                            writeln!(serial, " responded to 0x").ok();
                            crate::compat::ascii::write_bytes_hex_fmt(serial, &[seq_cmd]).ok();
                            writeln!(serial).ok();
                        }
                        cmd_responded_by_initial_device = true;
                    }
                }
                if cmd_responded_by_initial_device {
                    detected_cmds.push(seq_cmd).map_err(|_| {
                        crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
                    })?;
                }
            }
            Err(e) => {
                if let crate::explore::logger::LogLevel::Verbose = log_level {
                    write!(serial, "[error] scan failed for command 0x").ok();
                    crate::compat::ascii::write_bytes_hex_fmt(serial, &[seq_cmd]).ok();
                    writeln!(serial, ": {:?}", e).ok();
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

define_scanner!(
    crate::compat::I2cCompat,
    crate::compat::HalErrorExt,
    core::fmt::Write
);
