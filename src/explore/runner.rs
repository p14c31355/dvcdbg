// runner.rs

use crate::compat::util;
use crate::error::ExplorerError;
use crate::explore::explorer::*;
use crate::scanner::I2C_MAX_DEVICES;

#[macro_export]
macro_rules! factorial_sort {
    ($explorer:expr, $i2c:expr, $serial:expr, $prefix:expr, $init_sequence:expr, $n:expr, $init_len:expr, $cmd_buf:expr) => {
        $crate::explore::runner::factorial_explorer::<_, _, $n, $init_len, $cmd_buf>(
            $explorer, $i2c, $serial, $prefix, $init_sequence
        )
    };
}

pub fn factorial_explorer<
    I2C,
    S,
    const N: usize,
    const INIT_SEQUENCE_LEN: usize,
    const CMD_BUFFER_SIZE: usize,
>(
    explorer: &Explorer<N>,
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
    util::prevent_garbled(
        serial,
        format_args!("[explorer] Running full exploration..."),
    );

    let found_addrs = match crate::scanner::scan_i2c(i2c, serial, prefix) {
        Ok(addrs) => addrs,
        Err(e) => {
            util::prevent_garbled(serial, format_args!("[error] Failed to scan I2C: {e}"));
            return Err(ExplorerError::ExecutionFailed(e));
        }
    };
    if found_addrs.is_empty() {
        return Err(ExplorerError::NoValidAddressesFound);
    }

    let successful_seq = match crate::scanner::scan_init_sequence::<_, _, INIT_SEQUENCE_LEN>(
        i2c,
        serial,
        prefix,
        init_sequence,
    ) {
        Ok(seq) => seq,
        Err(e) => {
            util::prevent_garbled(
                serial,
                format_args!("[error] Failed to scan init sequence: {e}"),
            );
            return Err(ExplorerError::ExecutionFailed(e));
        }
    };
    util::prevent_garbled(
        serial,
        format_args!("[scan] initial sequence scan completed"),
    );
    util::prevent_garbled(serial, format_args!("[log] Start driver safe init"));

    let mut executor =
        PrefixExecutor::<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>::new(prefix, &successful_seq);

    let exploration_result =
        explorer.explore::<_, _, _, CMD_BUFFER_SIZE>(i2c, &mut executor, serial)?;

    for addr in exploration_result.found_addrs[..exploration_result.found_addrs_len].iter() {
        util::prevent_garbled(serial, format_args!("[driver] Found device at {addr:02X}"));
    }

    util::prevent_garbled(
        serial,
        format_args!(
            "[explorer] Exploration complete. {} addresses found across {} permutations.",
            exploration_result.found_addrs_len, exploration_result.permutations_tested
        ),
    );

    Ok(())
}

#[macro_export]
macro_rules! pruning_sort {
    ($explorer:expr, $i2c:expr, $serial:expr, $prefix:expr, $init_sequence:expr, $n:expr, $init_len:expr, $cmd_buf:expr) => {
        $crate::explore::runner::pruning_explorer::<_, _, $n, $init_len, $cmd_buf>(
            $explorer, $i2c, $serial, $prefix, $init_sequence
        )
    };
}

pub fn pruning_explorer<
    I2C,
    S,
    const N: usize,
    const INIT_SEQUENCE_LEN: usize,
    const CMD_BUFFER_SIZE: usize,
>(
    explorer: &Explorer<N>,
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
            return Err(ExplorerError::ExecutionFailed(e));
        }
    };
    if target_addrs.is_empty() {
        return Err(ExplorerError::NoValidAddressesFound);
    }

    let successful_seq: heapless::Vec<u8, INIT_SEQUENCE_LEN> =
        match crate::scanner::scan_init_sequence::<_, _, INIT_SEQUENCE_LEN>(
            i2c,
            serial,
            prefix,
            init_sequence,
        ) {
            Ok(seq) => seq,
            Err(e) => {
                util::prevent_garbled(serial, format_args!("Failed to scan init sequence: {e}"));
                return Err(ExplorerError::ExecutionFailed(e));
            }
        };

    let successful_seq_len = successful_seq.len();

    util::prevent_garbled(
        serial,
        format_args!("[scan] initial sequence scan completed"),
    );

    let mut executor =
        crate::explore::explorer::PrefixExecutor::<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>::new(
            target_addrs[0],
            &successful_seq,
        );

    let mut failed_nodes = [false; N];

    loop {
        let (sequence_bytes, _sequence_len) =
            match explorer.get_one_sort(serial, &failed_nodes) {
                Ok(seq) => seq,
                Err(ExplorerError::DependencyCycle) => {
                    util::prevent_garbled(
                        serial,
                        format_args!("[error] Dependency cycle detected, stopping exploration"),
                    );
                    break;
                }
                Err(e) => {
                    util::prevent_garbled(
                        serial,
                        format_args!("[error] Failed to generate topological sort: {e}. Aborting."),
                    );
                    return Err(e);
                }
            };

        let mut addrs_to_remove: heapless::Vec<usize, I2C_MAX_DEVICES> = heapless::Vec::new();

        for (addr_idx, &addr) in target_addrs.iter().enumerate() {
            util::prevent_garbled(serial, format_args!("Sending commands to {addr:02X}"));

            let mut all_ok = true;

            for i in 0..explorer.nodes.len() {
                if failed_nodes[i] {
                    continue;
                }
                let cmd_bytes = &sequence_bytes[i];

                if exec_logger(i2c, &mut executor, serial, addr, cmd_bytes, i).is_err()
                {
                    util::prevent_garbled(
                        serial,
                        format_args!("[warn] Command {i} failed on {addr:02X}"),
                    );
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
            target_addrs.swap_remove(idx);
        }

        if target_addrs.is_empty() || failed_nodes.iter().all(|&x| x) {
            break;
        }
    }

    util::prevent_garbled(serial, format_args!("[I] Explorer finished"));
    Ok(())
}

#[macro_export]
macro_rules! get_one_sort {
    ($explorer:expr, $i2c:expr, $serial:expr, $prefix:expr, $n:expr, $init_len:expr, $cmd_buf:expr) => {
        $crate::explore::runner::one_topological_explorer::<_, _, $n, $init_len, $cmd_buf>(
            $explorer, $i2c, $serial, $prefix
        )
    };
}

pub fn one_topological_explorer<
    I2C,
    S,
    const N: usize,
    const INIT_SEQUENCE_LEN: usize,
    const CMD_BUFFER_SIZE: usize,
>(
    explorer: &Explorer<N>,
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
            return Err(ExplorerError::ExecutionFailed(e));
        }
    };
    if target_addr.is_empty() {
        return Err(ExplorerError::NoValidAddressesFound);
    }

    let single_sequence = explorer.get_one_sort(serial, &[false; N])?;

    let sequence_len = explorer.nodes.len();

    util::prevent_garbled(
        serial,
        format_args!("[explorer] Obtained one topological sort. Executing on {:02X}...", target_addr[0]),
    );

    let empty_seq: &[u8] = &[];
    let mut executor = PrefixExecutor::<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>::new(prefix, empty_seq);

    for i in 0..sequence_len {
        exec_logger(
            i2c,
            &mut executor,
            serial,
            target_addr[0],
            single_sequence.0[i],
            i,
        )?;
    }

    util::prevent_garbled(
        serial,
        format_args!("[explorer] Single sequence execution complete for {:02X}.", target_addr[0]),
    );

    Ok(())
}
