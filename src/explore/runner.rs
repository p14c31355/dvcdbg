// runner.rs

use crate::compat::util;
use crate::error::ExplorerError;
use crate::explore::explorer::*;
use crate::scanner::I2C_MAX_DEVICES;

#[macro_export]
macro_rules! pruning_sort {
    ($explorer:expr, $i2c:expr, $serial:expr, $prefix:expr, $init_sequence:expr, $n:expr, $init_len:expr, $cmd_buf:expr, $max_deps:expr) => {
        $crate::explore::runner::pruning_explorer::<_, _, $n, $init_len, $cmd_buf, $max_deps>(
            $explorer,
            $i2c,
            $serial,
            $prefix,
            $init_sequence,
        )
    };
}

pub fn pruning_explorer<
    I2C,
    S,
    const N: usize,
    const INIT_SEQUENCE_LEN: usize,
    const CMD_BUFFER_SIZE: usize,
    const MAX_DEPS: usize,
>(
    explorer: &Explorer<N, MAX_DEPS>,
    i2c: &mut I2C,
    serial: &mut S,
    prefix: u8,
    _init_sequence: &[u8; INIT_SEQUENCE_LEN],
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let mut target_addrs = crate::scanner::scan_i2c(i2c, serial, prefix)?;

    if target_addrs.is_empty() {
        write!(serial, "[I] Init scan OK: No devices found\r\n").ok();
        return Err(ExplorerError::NoValidAddressesFound);
    }

    let mut failed_nodes = util::BitFlags::new();
    let mut executor = PrefixExecutor::<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>::new(prefix, &[]);
    loop {
        if target_addrs.is_empty() {
            write!(serial, "[I] All valid addresses explored. Done.\r\n").ok();
            return Ok(());
        }

        let mut addrs_to_remove = heapless::Vec::<usize, { I2C_MAX_DEVICES }>::new();

        for (addr_idx, &addr) in target_addrs.iter().enumerate() {
            core::fmt::Write::write_str(serial, "[I] RUN ON ").ok();
            crate::compat::util::write_bytes_hex_fmt(serial, &[addr]).ok();
            core::fmt::Write::write_str(serial, "\r\n").ok();

            let mut sort_iter = match explorer.topological_iter(&failed_nodes) {
                Ok(iter) => iter,
                Err(e) => {
                    write!(serial, "[E] Failed GEN topological sort: {e}\r\n").ok();
                    addrs_to_remove.push(addr_idx).ok();
                    continue;
                }
            };

            let mut command_to_fail = None;

            for cmd_idx in sort_iter.by_ref() {
                let cmd_bytes = explorer.nodes[cmd_idx].bytes;
                if super::explorer::exec_log_cmd(
                    i2c,
                    &mut executor,
                    serial,
                    addr,
                    cmd_bytes,
                    cmd_idx,
                )
                .is_err()
                {
                    write!(serial, "[warn] Command {cmd_idx} failed on {addr:02X}\r\n").ok();

                    command_to_fail = Some(cmd_idx);
                    break;
                }
            }

            let is_cycle_detected = sort_iter.is_cycle_detected();

            if is_cycle_detected {
                write!(serial, "[E] Dependency cycle detected. Aborting.\r\n").ok();
                return Err(ExplorerError::DependencyCycle);
            }

            if let Some(cmd_idx) = command_to_fail {
                failed_nodes.set(cmd_idx).unwrap();
            } else {
                addrs_to_remove.push(addr_idx).ok();
            }
        }

        for &idx in addrs_to_remove.iter().rev() {
            target_addrs.swap_remove(idx);
        }
    }
}

#[macro_export]
macro_rules! get_one_sort {
    ($explorer:expr, $i2c:expr, $serial:expr, $prefix:expr, $n:expr, $init_len:expr, $cmd_buf:expr, $max_deps:expr) => {
        $crate::explore::runner::one_topological_explorer::<_, _, $n, $init_len, $cmd_buf, $max_deps>(
            $explorer, $i2c, $serial, $prefix,
        )
    };
}

pub fn one_topological_explorer<
    I2C,
    S,
    const N: usize,
    const INIT_SEQUENCE_LEN: usize,
    const CMD_BUFFER_SIZE: usize,
    const MAX_DEPS: usize,
>(
    explorer: &Explorer<N, MAX_DEPS>,
    i2c: &mut I2C,
    serial: &mut S,
    prefix: u8,
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    core::fmt::Write::write_str(serial, "[exprore] Attempting to get 1 init seq ...\r\n").ok();

    let target_addr = match crate::scanner::scan_i2c(i2c, serial, prefix) {
        Ok(addr) => addr,
        Err(e) => {
            write!(serial, "[error] Failed to scan I2C: {e}\r\n").ok();
            return Err(ExplorerError::ExecutionFailed(e));
        }
    };
    if target_addr.is_empty() {
        // target_addr is a Vec<u8> here
        return Err(ExplorerError::NoValidAddressesFound);
    }

    let failed_nodes = util::BitFlags::new();
    let mut sort_iter = match explorer.topological_iter(&failed_nodes) {
        Ok(iter) => iter,
        Err(e) => {
            write!(
                serial,
                "[E] Failed to GEN topological sort: {e}. Aborting.\r\n"
            )
            .ok();
            return Err(e);
        }
    };

    core::fmt::Write::write_str(
        serial,
        "[explorer] Obtained one topological sort. Executing on ",
    )
    .ok();
    crate::compat::util::write_bytes_hex_fmt(serial, &[target_addr[0]]).ok();
    core::fmt::Write::write_str(serial, "...\r\n").ok();

    let empty_seq: &[u8] = &[];
    let mut executor = PrefixExecutor::<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>::new(prefix, empty_seq);

    for cmd_idx in sort_iter.by_ref() {
        super::explorer::exec_log_cmd(
            i2c,
            &mut executor,
            serial,
            target_addr[0],
            explorer.nodes[cmd_idx].bytes,
            cmd_idx,
        )?;
    }
    if sort_iter.is_cycle_detected() {
        core::fmt::Write::write_str(serial, "[error] Dependency cycle detected!\r\n").ok();
        return Err(ExplorerError::DependencyCycle);
    }

    core::fmt::Write::write_str(serial, "[explorer] Single sequence execution complete for ").ok();
    crate::compat::util::write_bytes_hex_fmt(serial, &[target_addr[0]]).ok();
    core::fmt::Write::write_str(serial, ".\r\n").ok();

    Ok(())
}
