use crate::explore::logger::*;
use crate::explore::explorer::*;

use crate::error::ExplorerError;
use crate::compat::ascii;

use core::fmt::Write;

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
            return Err(ExplorerError::ExecutionFailed);
        }
    };
    serial_logger.log_info_fmt(|buf| writeln!(buf, "[scan] initial sequence scan completed"));
    serial_logger.log_info_fmt(|buf| writeln!(buf, "[log] Start driver safe init"));

    let mut executor = PrefixExecutor::<BUF_CAP>::new(prefix, successful_seq);

    let exploration_result =
        explorer.explore::<_, _, _, BUF_CAP>(i2c, &mut executor, &mut serial_logger)?;

    for addr in exploration_result.found_addrs.iter() {
        write!(serial, "[driver] Found device at ").ok();
        ascii::write_bytes_hex_fmt(serial, &[*addr]).ok();
        writeln!(serial).ok();
    }

    Ok(())
}

pub fn run_pruned_explorer<I2C, S, E, const N: usize, const BUF_CAP: usize, const MAX_CMD_LEN: usize>(
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
    let mut found_addrs = match crate::scanner::scan_i2c(i2c, &mut serial_logger, &[prefix], log_level) {
        Ok(addrs) => addrs,
        Err(e) => return Err(ExplorerError::DeviceNotFound(e)),
    };
    if found_addrs.is_empty() {
        return Err(ExplorerError::NoValidAddressesFound);
    }
    let mut failed_nodes = [false; N];
    let mut commands_found = 0;
    loop {
        let (sequence_bytes, sequence_len) =
            match explorer.get_one_topological_sort_buf::<MAX_CMD_LEN>(&mut serial_logger, &failed_nodes) {
                Ok(seq) => seq,
                Err(e) => {
                    if commands_found == explorer.sequence.len() {
                        serial_logger.log_info("[explorer] All commands successfully executed.");
                        return Ok(());
                    } else {
                        serial_logger.log_error_fmt(|buf| {
                            writeln!(buf, "[error] Failed to generate a new topological sort. Aborting.")
                        });
                        return Err(e);
                    }
                }
            };
        let mut addrs_to_remove: heapless::Vec<usize, 128> = heapless::Vec::new();
        for (addr_idx, &addr) in found_addrs.iter().enumerate() {
            let mut all_ok = true;
            let mut current_failed_nodes = failed_nodes;
            for i in 0..explorer.sequence.len() {
                let cmd_bytes = &sequence_bytes[i][..sequence_len[i]];
                match executor.exec(i2c, addr, cmd_bytes, &mut serial_logger) {
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
                commands_found += explorer.sequence.len();
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

pub fn run_single_sequence_explorer<I2C, S, const N: usize, const BUF_CAP: usize, const MAX_CMD_LEN: usize>(
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
    serial_logger.log_info_fmt(|buf| {
        write!(
            buf,
            "[explorer] Attempting to get one topological sort...\r\n"
        )?;
        Ok(())
    });

    let single_sequence = explorer.get_one_topological_sort_buf::<MAX_CMD_LEN>(&mut serial_logger, &[false; N])?;
    serial_logger.log_info_fmt(|buf| writeln!(buf, "Before sort:"));
    for (idx, node) in explorer.sequence.iter().enumerate() {
        serial_logger.log_info_fmt(|buf| writeln!(buf, "Node {idx} deps: {:?}", node.deps));
    }

    let sequence_len = explorer.sequence.len();

    serial_logger.log_info_fmt(|buf| {
        writeln!(
            buf,
            "[explorer] Obtained one topological sort. Executing on 0x{target_addr:02X}..."
        )?;
        Ok(())
    });

    for node_idx in 0..explorer.sequence.len() {
        writeln!(serial_logger, "Checking node {node_idx}").ok();
    }

    let mut executor = PrefixExecutor::<BUF_CAP>::new(prefix, heapless::Vec::new());

    for i in 0..sequence_len {
        serial_logger.log_info_fmt(|buf| {
            writeln!(
                buf,
                "[explorer] Sending node {} bytes: {:02X?} ...",
                i, single_sequence.0[i]
            )?;
            Ok(())
        });
        match executor.exec(i2c, target_addr, &single_sequence.0[i], &mut serial_logger) {
            Ok(_) => {
                serial_logger.log_info_fmt(|buf| {
                    writeln!(buf, "OK")?;
                    Ok(())
                });
            }
            Err(e) => {
                serial_logger.log_error_fmt(|buf| {
                    writeln!(buf, "FAILED: {e:?}")?; // `e` is now in scope
                    Ok(())
                });
                return Err(e.into()); // Convert ExecutorError to ExplorerError and return
            }
        };
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