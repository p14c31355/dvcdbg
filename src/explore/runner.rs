// runner.rs

use crate::explore::explorer::*;
use crate::explore::logger::*;
use crate::compat::err_compat::HalErrorExt;
use crate::compat::util;
use crate::error::ExplorerError;
use core::fmt::Write;
use crate::compat::util::ERROR_STRING_BUFFER_SIZE;
use crate::scanner::I2C_MAX_DEVICES;
use crate::compat::util::calculate_cmd_buffer_size; // Import the const fn

// Helper for logging info with the [explorer] prefix
pub fn explorer_log_info<S, F>(logger: &mut SerialLogger<S>, f: F)
where
    S: core::fmt::Write,
    F: FnOnce(&mut heapless::String<ERROR_STRING_BUFFER_SIZE>) -> core::fmt::Result,
{
    logger.log_info_fmt(|buf| {
        write!(buf, "[explorer] ")?;
        f(buf)
    });
}

// Helper for logging errors with the [explorer] prefix
pub fn explorer_log_error<S, F>(logger: &mut SerialLogger<S>, f: F)
where
    S: core::fmt::Write,
    F: FnOnce(&mut heapless::String<ERROR_STRING_BUFFER_SIZE>) -> core::fmt::Result,
{
    logger.log_error_fmt(|buf| {
        write!(buf, "[explorer] ")?;
        f(buf)
    });
}

// Helper for logging errors with the [error] prefix (used in run_explorer and run_pruned_explorer)
pub fn runner_log_error<S, F>(logger: &mut SerialLogger<S>, f: F)
where
    S: core::fmt::Write,
    F: FnOnce(&mut heapless::String<ERROR_STRING_BUFFER_SIZE>) -> core::fmt::Result,
{
    logger.log_error_fmt(|buf| {
        write!(buf, "[error] ")?;
        f(buf)
    });
}

pub fn run_explorer<
    I2C,
    S,
    const N: usize,
    const INIT_SEQUENCE_LEN: usize,
    const CMD_BUFFER_SIZE: usize, // Add CMD_BUFFER_SIZE
>(
    explorer: &Explorer<'_, N>,
    i2c: &mut I2C,
    serial: &mut S,
    prefix: u8,
    init_sequence: &[u8; INIT_SEQUENCE_LEN], // Use INIT_SEQUENCE_LEN
    log_level: LogLevel,
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write + Logger,
{
    let mut serial_logger = SerialLogger::new(serial, log_level);
    explorer_log_info(&mut serial_logger, |buf| {
        writeln!(buf, "Running full exploration...")
    });

    let found_addrs = match crate::scanner::scan_i2c(i2c, &mut serial_logger, prefix) {
        Ok(addrs) => addrs,
        Err(e) => {
            runner_log_error(&mut serial_logger, |buf| {
                writeln!(buf, "Failed to scan I2C: {:?}", e)
            });
            return Err(ExplorerError::ExecutionFailed(e.into()));
        }
    };
    if found_addrs.is_empty() {
        return Err(ExplorerError::NoValidAddressesFound);
    }

    let successful_seq = match crate::scanner::scan_init_sequence::<_, _, INIT_SEQUENCE_LEN>( // Use INIT_SEQUENCE_LEN
        i2c,
        &mut serial_logger,
        prefix,
        init_sequence,
    ) {
        Ok(seq) => seq,
        Err(e) => {
            runner_log_error(&mut serial_logger, |buf| {
                writeln!(buf, "Failed to scan init sequence: {:?}", e)
            });
            return Err(ExplorerError::ExecutionFailed(e));
        }
    };
    serial_logger.log_info_fmt(|buf| writeln!(buf, "[scan] initial sequence scan completed"));
    serial_logger.log_info_fmt(|buf| writeln!(buf, "[log] Start driver safe init"));

    let mut executor =
        PrefixExecutor::<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>::new(prefix, successful_seq); // Use calculated size

    let exploration_result =
        explorer.explore::<_, _, _, CMD_BUFFER_SIZE>(i2c, &mut executor, &mut serial_logger)?; // Use CMD_BUFFER_SIZE

    for addr in exploration_result.found_addrs.iter() {
        serial_logger.log_info_fmt(|buf| {
            write!(buf, "[driver] Found device at ")?;
            util::write_bytes_hex_fmt(buf, &[*addr])?;
            writeln!(buf)
        });
    }

    serial_logger.log_info_fmt(|buf| {
        writeln!(
            buf,
            "[explorer] Exploration complete. {} addresses found across {} permutations.",
            exploration_result.found_addrs.len(),
            exploration_result.permutations_tested
        )
    });

    Ok(())
}

pub fn run_pruned_explorer<
    I2C,
    S,
    const N: usize,
    const INIT_SEQUENCE_LEN: usize,
    const CMD_BUFFER_SIZE: usize, // Add CMD_BUFFER_SIZE
>(
    explorer: &Explorer<'_, N>,
    i2c: &mut I2C,
    serial: &mut S,
    prefix: u8,
    init_sequence: &[u8; INIT_SEQUENCE_LEN], // Use INIT_SEQUENCE_LEN
    log_level: LogLevel,
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let mut serial_logger = SerialLogger::new(serial, log_level);

    let mut found_addrs = match crate::scanner::scan_i2c(i2c, &mut serial_logger, prefix) {
        Ok(addrs) => addrs,
        Err(e) => {
            runner_log_error(&mut serial_logger, |buf| {
                writeln!(buf, "Failed to scan I2C: {:?}", e)
            });
            return Err(ExplorerError::ExecutionFailed(e.into()));
        }
    };
    if found_addrs.is_empty() {
        return Err(ExplorerError::NoValidAddressesFound);
    }

    let successful_seq: heapless::Vec<u8, INIT_SEQUENCE_LEN> = // Use INIT_SEQUENCE_LEN
        match crate::scanner::scan_init_sequence::<_, _, INIT_SEQUENCE_LEN>(i2c, &mut serial_logger, prefix, init_sequence) { // Use INIT_SEQUENCE_LEN
            Ok(seq) => seq,
            Err(e) => {
                serial_logger
                    .log_error_fmt(|buf| writeln!(buf, "Failed to scan init sequence: {:?}", e));
                return Err(ExplorerError::ExecutionFailed(e.into()));
            }
        };

    let successful_seq_len = successful_seq.len();

    serial_logger.log_info_fmt(|buf| writeln!(buf, "[scan] initial sequence scan completed"));

    let mut executor =
        crate::explore::explorer::PrefixExecutor::<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>::new( // Use calculated size
            found_addrs[0],
            successful_seq,
        );

    let mut failed_nodes = [false; N];

    loop {
        let (sequence_bytes, _sequence_len) = match explorer
        .get_one_topological_sort_buf(&mut serial_logger, &failed_nodes) // No generic needed here
    {
                Ok(seq) => seq,
                Err(ExplorerError::DependencyCycle) => {
                    serial_logger.log_error_fmt(|buf| {
                        writeln!(
                            buf,
                            "[error] Dependency cycle detected, stopping exploration"
                        )
                    });
                    break;
                }
                Err(e) => {
                    serial_logger.log_error_fmt(|buf| {
                        writeln!(
                            buf,
                            "[error] Failed to generate topological sort: {:?}. Aborting.",
                            e
                        )
                    });
                    return Err(e);
                }
            };

        let mut addrs_to_remove: heapless::Vec<usize, I2C_MAX_DEVICES> = heapless::Vec::new();

        for (addr_idx, &addr) in found_addrs.iter().enumerate() {
            serial_logger.log_info_fmt(|buf| write!(buf, "Sending commands to 0x{:02X}", addr));

            let mut all_ok = true;

            for i in 0..explorer.sequence.len() {
                if failed_nodes[i] {
                    continue;
                }
                let cmd_bytes = &sequence_bytes[i];

                if execute_and_log_command(
                    i2c,
                    &mut executor,
                    &mut serial_logger,
                    addr,
                    cmd_bytes,
                    i,
                )
                .is_err()
                {
                    serial_logger.log_info_fmt(|buf| {
                        write!(buf, "[warn] Command {} failed on 0x{:02X}", i, addr)
                    });
                    all_ok = false;
                    if i >= successful_seq_len {
                        failed_nodes[i] = true;
                    }
                    break;
                }
            }

            if all_ok {
                addrs_to_remove.push(addr_idx).ok();
            }
        }

        for &idx in addrs_to_remove.iter().rev() {
            found_addrs.swap_remove(idx);
        }

        if found_addrs.is_empty() || failed_nodes.iter().all(|&x| x) {
            break;
        }
    }

    serial_logger.log_info_fmt(|buf| writeln!(buf, "[I] Explorer finished"));
    Ok(())
}

pub fn run_single_sequence_explorer<
    I2C,
    S,
    const N: usize,
    const INIT_SEQUENCE_LEN: usize,
    const CMD_BUFFER_SIZE: usize, // Add CMD_BUFFER_SIZE
>(
    explorer: &Explorer<'_, N>,
    i2c: &mut I2C,
    serial: &mut S,
    prefix: u8,
    target_addr: u8,
    log_level: LogLevel,
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let mut serial_logger = SerialLogger::new(serial, log_level);
    explorer_log_info(&mut serial_logger, |buf| {
        writeln!(buf, "Attempting to get one topological sort...")
    });

    let single_sequence = explorer.get_one_topological_sort_buf(&mut serial_logger, &[false; N])?; // No generic needed here

    let sequence_len = explorer.sequence.len();

    serial_logger.log_info_fmt(|buf| {
        writeln!(
            buf,
            "[explorer] Obtained one topological sort. Executing on 0x{target_addr:02X}..."
        )?;
        Ok(())
    });

    let mut executor = PrefixExecutor::<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>::new(prefix, heapless::Vec::new()); // Use calculated size

    for i in 0..sequence_len {
        execute_and_log_command(
            i2c,
            &mut executor,
            &mut serial_logger,
            target_addr,
            &single_sequence.0[i],
            i,
        )?;
    }

    serial_logger.log_info_fmt(|buf| {
        writeln!(
            buf,
            "[explorer] Single sequence execution complete for 0x{target_addr:02X}."
        )?;
        Ok(())
    });

    Ok(())
}
