// runner.rs

use crate::compat::util::calculate_cmd_buffer_size;
use crate::explore::explorer::*;
use crate::explore::logger::*;

use crate::compat::util;
use crate::error::ExplorerError;

use core::fmt::Write;

use crate::compat::util::ERROR_STRING_BUFFER_SIZE;
use crate::scanner::I2C_MAX_DEVICES;

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

pub fn run_explorer<I2C, S, const N: usize, const MAX_CMD_LEN: usize>(
    explorer: &Explorer<'_, N>,
    i2c: &mut I2C,
    serial: &mut S,
    target_addr: u8,
    prefix: u8,
    init_sequence: &[u8; N],
    log_level: LogLevel,
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let mut serial_logger = SerialLogger::new(serial, log_level);
    explorer_log_info(&mut serial_logger, |buf| {
        writeln!(buf, "Running full exploration...")
    });

    let successful_seq = match crate::scanner::scan_init_sequence(
        i2c,
        &mut serial_logger,
        prefix,
        init_sequence,
        log_level,
    ) {
        Ok(seq) => seq,
        Err(e) => {
            runner_log_error(&mut serial_logger, |buf| {
                writeln!(buf, "Failed to scan init sequence: {:?}", e)
            });
            return Err(ExplorerError::DeviceNotFound(e));
        }
    };
    serial_logger.log_info_fmt(|buf| writeln!(buf, "[scan] initial sequence scan completed"));
    serial_logger.log_info_fmt(|buf| writeln!(buf, "[log] Start driver safe init"));

    let buf_cap: usize = calculate_cmd_buffer_size(1, explorer.max_cmd_len());

    let mut executor = PrefixExecutor::<N, MAX_CMD_LEN>::new(prefix, successful_seq);

    let exploration_result =
        explorer.explore::<_, _, _, MAX_CMD_LEN>(i2c, &mut executor, &mut serial_logger)?;

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

pub fn run_pruned_explorer<I2C, S, const N: usize, const MAX_CMD_LEN: usize>(
    explorer: &Explorer<'_, N>,
    i2c: &mut I2C,
    serial: &mut S,
    prefix: u8,
    init_sequence: &[u8; MAX_CMD_LEN],
    log_level: LogLevel,
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let max_len = explorer.max_cmd_len();
    let mut serial_logger = SerialLogger::new(serial, log_level);
    let mut found_addrs = match crate::scanner::scan_i2c(i2c, &mut serial_logger, prefix, log_level)
    {
        Ok(addrs) => addrs,
        Err(e) => return Err(ExplorerError::DeviceNotFound(e)),
    };
    if found_addrs.is_empty() {
        return Err(ExplorerError::NoValidAddressesFound);
    }
    let successful_seq = match crate::scanner::scan_init_sequence(
        i2c,
        &mut serial_logger,
        prefix,
        init_sequence,
        log_level,
    ) {
        Ok(seq) => seq,
        Err(e) => {
            runner_log_error(&mut serial_logger, |buf| {
                writeln!(buf, "Failed to scan init sequence: {:?}", e)
            });
            return Err(ExplorerError::DeviceNotFound(e));
        }
    };
    serial_logger.log_info_fmt(|buf| writeln!(buf, "[scan] initial sequence scan completed"));
    serial_logger.log_info_fmt(|buf| writeln!(buf, "[log] Start driver safe init"));
    let mut executor = crate::explore::explorer::PrefixExecutor::<MAX_CMD_LEN, MAX_CMD_LEN>::new(
        prefix,
        successful_seq,
    );

    let mut failed_nodes = [false; N];
    loop {
        let (sequence_bytes, _sequence_len) = match explorer
            .get_one_topological_sort_buf::<MAX_CMD_LEN>(&mut serial_logger, &failed_nodes)
        {
            Ok(seq) => seq,
            Err(e) => {
                serial_logger.log_error_fmt(|buf| {
                    writeln!(
                        buf,
                        "[error] Failed to generate a new topological sort: {:?}. Aborting.",
                        e
                    )
                });
                return Err(e);
            }
        };
        let mut addrs_to_remove: heapless::Vec<usize, I2C_MAX_DEVICES> = heapless::Vec::new();
        for (addr_idx, &addr) in found_addrs.iter().enumerate() {
            let mut all_ok = true;
            let mut current_failed_nodes = failed_nodes;
            for i in 0..explorer.sequence.len() {
                let cmd_bytes = &sequence_bytes[i];
                match execute_and_log_command(
                    i2c,
                    &mut executor,
                    &mut serial_logger,
                    addr,
                    cmd_bytes,
                    i,
                ) {
                    Ok(_) => {}
                    Err(_) => {
                        current_failed_nodes[i] = true;
                        all_ok = false;
                        break;
                    }
                }
            }
            if all_ok {
                addrs_to_remove.push(addr_idx).ok();
            }
            failed_nodes = current_failed_nodes;
        }
        for &idx in addrs_to_remove.iter().rev() {
            found_addrs.swap_remove(idx);
        }
        if found_addrs.is_empty() {
            break;
        }
        let all_nodes_visited = failed_nodes.iter().all(|&x| x);
        if all_nodes_visited {
            break;
        }
    }
    Ok(())
}

pub fn run_single_sequence_explorer<I2C, S, const N: usize, const MAX_CMD_LEN: usize>(
    explorer: &Explorer<'_, N>,
    i2c: &mut I2C,
    serial: &mut S,
    target_addr: u8,
    prefix: u8,
    log_level: LogLevel,
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let max_len = explorer.max_cmd_len();
    let mut serial_logger = SerialLogger::new(serial, log_level);
    explorer_log_info(&mut serial_logger, |buf| {
        writeln!(buf, "Attempting to get one topological sort...")
    });

    let single_sequence =
        explorer.get_one_topological_sort_buf::<MAX_CMD_LEN>(&mut serial_logger, &[false; N])?;

    let sequence_len = explorer.sequence.len();

    serial_logger.log_info_fmt(|buf| {
        writeln!(
            buf,
            "[explorer] Obtained one topological sort. Executing on 0x{target_addr:02X}..."
        )?;
        Ok(())
    });

    let buf_cap: usize = calculate_cmd_buffer_size(1, explorer.max_cmd_len());
    let mut executor = PrefixExecutor::<N, MAX_CMD_LEN>::new(prefix, heapless::Vec::new());

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
