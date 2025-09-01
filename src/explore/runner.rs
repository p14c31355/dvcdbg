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
    init_sequence: &[u8; INIT_SEQUENCE_LEN],
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let mut target_addrs = match crate::scanner::scan_i2c(i2c, serial, prefix) {
        Ok(addrs) => addrs,
        Err(e) => {
            util::prevent_garbled(serial, format_args!("[error] Failed to scan I2C: {e:?}"));
            return Err(e);
        }
    };
    if target_addrs.is_empty() {
        return Err(ExplorerError::NoValidAddressesFound);
    }

    let successful_seq: heapless::Vec<u8, INIT_SEQUENCE_LEN> =
        match crate::scanner::scan_init_sequence::<_, _, INIT_SEQUENCE_LEN>(
            i2c,
            serial,
            init_sequence,
        ) {
            Ok(seq) => seq,
            Err(e) => {
                util::prevent_garbled(serial, format_args!("Failed to scan init sequence: {e}"));
                return Err(ExplorerError::ExecutionFailed(e));
            }
        };

    let _successful_seq_len = successful_seq.len();

    util::prevent_garbled(
        serial,
        format_args!("[scan] initial sequence scan completed"),
    );

    let mut executor =
        crate::explore::explorer::PrefixExecutor::<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>::new(
            target_addrs[0],
            &successful_seq,
        );

    let mut failed_nodes = util::BitFlags::new();
    let num_nodes = explorer.nodes.len();

    loop {
        let mut addrs_to_remove: heapless::Vec<usize, I2C_MAX_DEVICES> = heapless::Vec::new();

        for (addr_idx, &addr) in target_addrs.iter().enumerate() {
            util::prevent_garbled(serial, format_args!("Sending commands to {addr:02X}"));

            let mut all_ok = true;
            let mut command_to_fail: Option<usize> = None;

            let mut sort_iter = match explorer.topological_iter(&failed_nodes) {
                Ok(iter) => iter,
                Err(e) => {
                    util::prevent_garbled(
                        serial,
                        format_args!("[error] Failed to generate topological sort: {e}. Aborting."),
                    );
                    return Err(e);
                }
            };
            for cmd_idx in sort_iter.by_ref() {
                let cmd_bytes = explorer.nodes[cmd_idx].bytes;
                if exec_log_cmd(i2c, &mut executor, serial, addr, cmd_bytes, cmd_idx).is_err() {
                    util::prevent_garbled(
                        serial,
                        format_args!("[warn] Command {cmd_idx} failed on {addr:02X}"),
                    );
                    all_ok = false;
                    command_to_fail = Some(cmd_idx);
                    break;
                }
            }

            let is_cycle_detected = if all_ok {
                sort_iter.is_cycle_detected()
            } else {
                false
            };

            if let Some(cmd_idx) = command_to_fail {
                failed_nodes.set(cmd_idx).ok();
            }

            if is_cycle_detected {
                util::prevent_garbled(
                    serial,
                    format_args!(
                        "[error] Dependency cycle detected on {addr:02X}, stopping exploration for this address"
                    ),
                );
            } else if all_ok {
                addrs_to_remove.push(addr_idx).ok();
            }
        }

        for &idx in addrs_to_remove.iter().rev() {
            target_addrs.swap_remove(idx);
        }

        let all_failed = (0..num_nodes).all(|i| failed_nodes.get(i).unwrap_or(false));
        if target_addrs.is_empty() || all_failed {
            break;
        }
    }

    util::prevent_garbled(serial, format_args!("[I] Explorer finished"));
    Ok(())
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
    util::prevent_garbled(
        serial,
        format_args!("[explorer] Attempting to get one topological sort..."),
    );

    let target_addr = match crate::scanner::scan_i2c(i2c, serial, prefix) {
        Ok(addr) => addr,
        Err(e) => {
            util::prevent_garbled(serial, format_args!("[error] Failed to scan I2C: {e:?}"));
            return Err(e);
        }
    };
    if target_addr.is_empty() {
        return Err(ExplorerError::NoValidAddressesFound);
    }

    let failed_nodes = util::BitFlags::new();
    let mut sort_iter = explorer.topological_iter(&failed_nodes)?;

    util::prevent_garbled(
        serial,
        format_args!(
            "[explorer] Obtained one topological sort. Executing on {:02X}...",
            target_addr[0]
        ),
    );

    let empty_seq: &[u8] = &[];
    let mut executor = PrefixExecutor::<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>::new(prefix, empty_seq);

    for cmd_idx in sort_iter.by_ref() {
        exec_log_cmd(
            i2c,
            &mut executor,
            serial,
            target_addr[0],
            explorer.nodes[cmd_idx].bytes,
            cmd_idx,
        )?;
    }
    if sort_iter.is_cycle_detected() {
        util::prevent_garbled(serial, format_args!("[error] Dependency cycle detected!"));
        return Err(ExplorerError::DependencyCycle);
    }

    util::prevent_garbled(
        serial,
        format_args!(
            "[explorer] Single sequence execution complete for {:02X}.",
            target_addr[0]
        ),
    );

    Ok(())
}
