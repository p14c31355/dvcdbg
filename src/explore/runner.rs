use crate::explore::explorer::*;
use crate::explore::logger::*;

use crate::compat::ascii;
use crate::error::ExplorerError;

use core::fmt::Write;

use crate::scanner::{I2C_BUFFER_SIZE, I2C_MAX_DEVICES};

// Helper for logging info with the [explorer] prefix
pub fn explorer_log_info<S, F, const BUF_CAP: usize>(logger: &mut SerialLogger<S, BUF_CAP>, f: F)
where
    S: core::fmt::Write,
    F: FnOnce(&mut heapless::String<BUF_CAP>) -> core::fmt::Result,
{
    logger.log_info_fmt(|buf| {
        write!(buf, "[explorer] ")?;
        f(buf)
    });
}

// Helper for logging errors with the [explorer] prefix
pub fn explorer_log_error<S, F, const BUF_CAP: usize>(logger: &mut SerialLogger<S, BUF_CAP>, f: F)
where
    S: core::fmt::Write,
    F: FnOnce(&mut heapless::String<BUF_CAP>) -> core::fmt::Result,
{
    logger.log_error_fmt(|buf| {
        write!(buf, "[explorer] ")?;
        f(buf)
    });
}

// Helper for logging errors with the [error] prefix (used in run_explorer and run_pruned_explorer)
pub fn runner_log_error<S, F, const BUF_CAP: usize>(logger: &mut SerialLogger<S, BUF_CAP>, f: F)
where
    S: core::fmt::Write,
    F: FnOnce(&mut heapless::String<BUF_CAP>) -> core::fmt::Result,
{
    logger.log_error_fmt(|buf| {
        write!(buf, "[error] ")?;
        f(buf)
    });
}

pub fn run_explorer<I2C, S, const N: usize, const BUF_CAP: usize>(
    explorer: &Explorer<'_, N>,
    i2c: &mut I2C,
    serial: &mut S,
    init_sequence: &[u8],
    prefix: u8,
    log_level: LogLevel,
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let mut serial_logger = SerialLogger::new(serial, log_level);

    serial_logger.log_info_fmt(|buf| writeln!(buf, "[log] Initial I2C bus scan..."));

    let successful_seq = match crate::scanner::scan_init_sequence(
        i2c,
        &mut serial_logger,
        prefix,
        init_sequence,
        log_level,
    ) {
        Ok(seq) => seq,
        Err(e) => {
            serial_logger.log_error_fmt(|buf| {
                writeln!(
                    buf,
                    "[error] Initial sequence scan failed: {e:?}. Aborting explorer."
                )
            });
            return Err(ExplorerError::ExecutionFailed(e));
        }
    };
    serial_logger.log_info_fmt(|buf| writeln!(buf, "[scan] initial sequence scan completed"));
    serial_logger.log_info_fmt(|buf| writeln!(buf, "[log] Start driver safe init"));

    let mut executor = PrefixExecutor::<BUF_CAP>::new(prefix, successful_seq);

    let exploration_result =
        explorer.explore::<_, _, _, BUF_CAP>(i2c, &mut executor, &mut serial_logger)?;

    for addr in exploration_result.found_addrs.iter() {
        serial_logger.log_info_fmt(|buf| {
            write!(buf, "[driver] Found device at ")?;
            ascii::write_bytes_hex_fmt(buf, &[*addr])?;
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
    E,
    const N: usize,
    const BUF_CAP: usize,
    const MAX_CMD_LEN: usize,
>(
    explorer: &Explorer<'_, N>,
    i2c: &mut I2C,
    executor: &mut E,
    serial: &mut S,
    prefix: u8,
    log_level: LogLevel,
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    E: CmdExecutor<I2C, BUF_CAP>,
    S: core::fmt::Write + Logger<BUF_CAP>,
{
    let mut serial_logger = SerialLogger::new(serial, log_level);
    let mut found_addrs =
        match crate::scanner::scan_i2c(i2c, &mut serial_logger, &[prefix], log_level) {
            Ok(addrs) => addrs,
            Err(e) => return Err(ExplorerError::DeviceNotFound(e)),
        };
    if found_addrs.is_empty() {
        return Err(ExplorerError::NoValidAddressesFound);
    }
    let mut failed_nodes = [false; N];
    loop {
        let (sequence_bytes, sequence_len) = match explorer
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
        let mut addrs_to_remove: heapless::Vec<usize, I2C_BUFFER_SIZE> = heapless::Vec::new();
        for (addr_idx, &addr) in found_addrs.iter().enumerate() {
            let mut all_ok = true;
            let mut current_failed_nodes = failed_nodes;
            for i in 0..explorer.sequence.len() {
                let cmd_bytes = &sequence_bytes[i][..sequence_len[i]];
                match execute_and_log_command(i2c, executor, &mut serial_logger, addr, cmd_bytes, i)
                {
                    Ok(_) => {}
                    Err(_) => {
                        // The helper already logs the error and converts it
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

pub fn run_single_sequence_explorer<
    I2C,
    S,
    const N: usize,
    const BUF_CAP: usize,
    const MAX_CMD_LEN: usize,
>(
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

    let mut executor = PrefixExecutor::<BUF_CAP>::new(prefix, heapless::Vec::new());

    for i in 0..sequence_len {
        execute_and_log_command(
            i2c,
            &mut executor,
            &mut serial_logger,
            target_addr,
            &single_sequence.0[i],
            i,
        )?; // Propagate error
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
