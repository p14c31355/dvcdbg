//! Scanner utilities for I2C bus device discovery and analysis.

use crate::compat::HalErrorExt;
use crate::error::ExplorerError;
use crate::explore::logger::Logger;
use core::fmt::Write;

pub const I2C_SCAN_ADDR_START: u8 = 0x03;
pub const I2C_SCAN_ADDR_END: u8 = 0x77;
pub const I2C_MAX_DEVICES: usize = 128;

/// Scans the I2C bus for devices that respond to a given data write.
///
/// It iterates through all possible I2C addresses and attempts to write the
/// provided `data`.
fn internal_scan<I2C, L>(
    i2c: &mut I2C,
    logger: &mut L,
    data: &[u8],
) -> Result<heapless::Vec<u8, I2C_MAX_DEVICES>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    L: Logger,
{
    let mut found_addrs = heapless::Vec::<u8, I2C_MAX_DEVICES>::new();
    let mut last_error: Option<crate::error::ErrorKind> = None;

    for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
        logger.log_info_fmt(|buf| write!(buf, "Scanning 0x{:02x}...", addr));

        match i2c.write(addr, data) {
            Ok(_) => {
                found_addrs.push(addr).map_err(|_| {
                    crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
                })?;
                logger.log_info_fmt(|buf| write!(buf, "Found"));
            }
            Err(e) => {
                let error_kind = e.to_compat(Some(addr));
                if error_kind == crate::error::ErrorKind::I2c(crate::error::I2cError::Nack) {
                    logger.log_info_fmt(|buf| write!(buf, "No response (NACK)"));
                    continue;
                }
                logger.log_error_fmt(|buf| {
                    write!(buf, "Write failed at 0x{:02x}: {}", addr, error_kind)
                });
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
pub fn scan_i2c<I2C, L>(
    i2c: &mut I2C,
    logger: &mut L,
    ctrl_byte: u8,
) -> Result<heapless::Vec<u8, I2C_MAX_DEVICES>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    L: Logger,
{
    logger.log_info_fmt(|buf| write!(buf, "Scanning I2C bus with a single control byte..."));
    internal_scan(i2c, logger, &[ctrl_byte])
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
pub fn scan_init_sequence<I2C, L, const MAX_CMD_LEN: usize>(
    i2c: &mut I2C,
    logger: &mut L,
    ctrl_byte: u8,
    init_sequence: &[u8; MAX_CMD_LEN],
) -> Result<heapless::Vec<u8, MAX_CMD_LEN>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    L: Logger,
{
    logger.log_info_fmt(|buf| write!(buf, "Starting I2C bus scan with initialization sequence..."));
    logger.log_info_fmt(|buf| {
        write!(
            buf,
            "Initializing scan with control byte 0x{:02x}",
            ctrl_byte
        )
    });

    let found_addrs = match crate::scanner::scan_i2c(i2c, logger, ctrl_byte) {
        Ok(addrs) => addrs,
        Err(e) => {
            logger.log_error_fmt(|buf| write!(buf, "Failed to scan I2C: {:?}\r\n", e));
            return Err(e);
        }
    };

    if found_addrs.is_empty() {
        logger.log_error_fmt(|buf| write!(buf, "No I2C devices found.\r\n"));
        loop {}
    }

    let mut detected_cmds =
        check_init_sequence(i2c, logger, ctrl_byte, init_sequence, &found_addrs)?;

    logger.log_info_fmt(|buf| write!(buf, "I2C scan with init sequence complete."));
    log_sequence_summary(logger, init_sequence, &mut detected_cmds);

    Ok(detected_cmds)
}

fn check_init_sequence<I2C, L, const N: usize>(
    i2c: &mut I2C,
    logger: &mut L,
    ctrl_byte: u8,
    init_sequence: &[u8; N],
    found_addrs: &heapless::Vec<u8, I2C_MAX_DEVICES>,
) -> Result<heapless::Vec<u8, N>, crate::error::ErrorKind>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    L: Logger,
{
    let mut detected_cmds = heapless::Vec::<u8, N>::new();
    let mut last_error: Option<crate::error::ErrorKind> = None;

    for &addr in found_addrs.iter() {
        logger.log_info_fmt(|buf| write!(buf, "Testing init sequence on 0x{:02x}...", addr));

        for &cmd in init_sequence.iter() {
            let mut command_data = heapless::Vec::<u8, 2>::new(); // ctrl_byte + cmd
            command_data.push(ctrl_byte).map_err(|_| {
                crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
            })?;
            command_data.push(cmd).map_err(|_| {
                crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
            })?;

            logger.log_info_fmt(|buf| write!(buf, "  Sending command 0x{:02x} to 0x{:02x}...", cmd, addr));

            match i2c.write(addr, &command_data) {
                Ok(_) => {
                    if !detected_cmds.contains(&cmd) {
                        detected_cmds.push(cmd).map_err(|_| {
                            crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
                        })?;
                    }
                    logger.log_info_fmt(|buf| write!(buf, "  Command 0x{:02x} responded.", cmd));
                }
                Err(e) => {
                    let error_kind = e.to_compat(Some(addr));
                    if error_kind == crate::error::ErrorKind::I2c(crate::error::I2cError::Nack) {
                        logger.log_info_fmt(|buf| write!(buf, "  Command 0x{:02x} no response (NACK).", cmd));
                        continue;
                    }
                    logger.log_error_fmt(|buf| {
                        write!(buf, "  Write failed for 0x{:02x} at 0x{:02x}: {}", cmd, addr, error_kind)
                    });
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

fn log_sequence_summary<L: Logger, const N: usize>(
    logger: &mut L,
    expected_sequence: &[u8; N],
    detected_cmds: &mut heapless::Vec<u8, N>,
) {
    let mut missing_cmds = heapless::Vec::<u8, N>::new();
    let mut sorted_detected_cmds = detected_cmds.clone();
    sorted_detected_cmds.sort_unstable();

    for &cmd in expected_sequence.iter() {
        if sorted_detected_cmds.binary_search(&cmd).is_err() {
            missing_cmds.push(cmd).ok();
        }
    }

    logger.log_info_fmt(|buf| {
        write!(buf, "\n--- I2C Sequence Scan Summary ---")?;
        Ok(())
    });
    logger.log_info_fmt(|buf| {
        write!(buf, "Expected Commands:")?;
        for &cmd in expected_sequence {
            write!(buf, " 0x{:02x}", cmd)?;
        }
        writeln!(buf)?;
        Ok(())
    });

    logger.log_info_fmt(|buf| {
        write!(buf, "Commands That Responded:")?;
        for &cmd in detected_cmds.iter() {
            write!(buf, " 0x{:02x}", cmd)?;
        }
        writeln!(buf)?;
        Ok(())
    });

    logger.log_info_fmt(|buf| {
        write!(buf, "Commands With No Response:")?;
        for &cmd in missing_cmds.iter() {
            write!(buf, " 0x{:02x}", cmd)?;
        }
        writeln!(buf)?;
        Ok(())
    });

    logger.log_info_fmt(|buf| {
        write!(buf, "--- End Summary ---\n")?;
        Ok(())
    });
}
