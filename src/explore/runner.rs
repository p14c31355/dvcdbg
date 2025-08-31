// runner.rs

use crate::compat::util;
use crate::error::ExplorerError;
use crate::explore::explorer::*;
use crate::scanner::I2C_MAX_DEVICES;


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
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    writeln!(serial, "[explorer] Running full exploration...").ok();

    let found_addrs = match crate::scanner::scan_i2c(i2c, serial, prefix) {
        Ok(addrs) => addrs,
        Err(e) => {
            writeln!(serial, "[error] Failed to scan I2C: {}", e).ok();
            return Err(ExplorerError::ExecutionFailed(e.into()));
        }
    };
    if found_addrs.is_empty() {
        return Err(ExplorerError::NoValidAddressesFound);
    }

    let successful_seq = match crate::scanner::scan_init_sequence::<_, _, INIT_SEQUENCE_LEN>(
        // Use INIT_SEQUENCE_LEN
        i2c,
        serial,
        prefix,
        init_sequence,
    ) {
        Ok(seq) => seq,
        Err(e) => {
            writeln!(serial, "[error] Failed to scan init sequence: {}", e).ok();
            return Err(ExplorerError::ExecutionFailed(e));
        }
    };
    writeln!(serial, "[scan] initial sequence scan completed").ok();
    writeln!(serial, "[log] Start driver safe init").ok();

    let mut executor =
        PrefixExecutor::<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>::new(prefix, successful_seq); // Use calculated size

    let exploration_result =
        explorer.explore::<_, _, _, CMD_BUFFER_SIZE>(i2c, &mut executor, serial)?; // Use CMD_BUFFER_SIZE

    for addr in exploration_result.found_addrs.iter() {
        write!(serial, "[driver] Found device at ").ok();
        util::write_bytes_hex_fmt(serial, &[*addr]).ok();
        writeln!(serial).ok();
    }

    writeln!(
        serial,
        "[explorer] Exploration complete. {} addresses found across {} permutations.",
        exploration_result.found_addrs.len(),
        exploration_result.permutations_tested
    ).ok();

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
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let mut found_addrs = match crate::scanner::scan_i2c(i2c, serial, prefix) {
        Ok(addrs) => addrs,
        Err(e) => {
            writeln!(serial, "[error] Failed to scan I2C: {:?}", e).ok();
            return Err(ExplorerError::ExecutionFailed(e.into()));
        }
    };
    if found_addrs.is_empty() {
        return Err(ExplorerError::NoValidAddressesFound);
    }

    let successful_seq: heapless::Vec<u8, INIT_SEQUENCE_LEN> = // Use INIT_SEQUENCE_LEN
        match crate::scanner::scan_init_sequence::<_, _, INIT_SEQUENCE_LEN>(i2c, serial, prefix, init_sequence) { // Use INIT_SEQUENCE_LEN
            Ok(seq) => seq,
            Err(e) => {
                writeln!(serial, "Failed to scan init sequence: {}", e).ok();
                return Err(ExplorerError::ExecutionFailed(e.into()));
            }
        };

    let successful_seq_len = successful_seq.len();

    writeln!(serial, "[scan] initial sequence scan completed").ok();

    let mut executor =
        crate::explore::explorer::PrefixExecutor::<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>::new(
            // Use calculated size
            found_addrs[0],
            successful_seq,
        );

    let mut failed_nodes = [false; N];

    loop {
        let (sequence_bytes, _sequence_len) = match explorer
        .get_one_topological_sort_buf(serial, &failed_nodes) // No generic needed here
    {
                Ok(seq) => seq,
                Err(ExplorerError::DependencyCycle) => {
                    writeln!(
                        serial,
                        "[error] Dependency cycle detected, stopping exploration"
                    ).ok();
                    break;
                }
                Err(e) => {
                    writeln!(
                        serial,
                        "[error] Failed to generate topological sort: {}. Aborting.",
                        e
                    ).ok();
                    return Err(e);
                }
            };

        let mut addrs_to_remove: heapless::Vec<usize, I2C_MAX_DEVICES> = heapless::Vec::new();

        for (addr_idx, &addr) in found_addrs.iter().enumerate() {
            write!(serial, "Sending commands to ").ok();
            util::write_bytes_hex_fmt(serial, &[addr]).ok();
            writeln!(serial).ok();

            let mut all_ok = true;

            for i in 0..explorer.sequence.len() {
                if failed_nodes[i] {
                    continue;
                }
                let cmd_bytes = &sequence_bytes[i];

                if execute_and_log_command(
                    i2c,
                    &mut executor,
                    serial,
                    addr,
                    cmd_bytes,
                    i,
                )
                .is_err()
                {
                    write!(serial, "[warn] Command {} failed on ", i).ok();
                    util::write_bytes_hex_fmt(serial, &[addr]).ok();
                    writeln!(serial).ok();
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

    writeln!(serial, "[I] Explorer finished").ok();
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
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    writeln!(serial, "[explorer] Attempting to get one topological sort...").ok();

    let single_sequence = explorer.get_one_topological_sort_buf(serial, &[false; N])?; // No generic needed here

    let sequence_len = explorer.sequence.len();

    write!(serial, "[explorer] Obtained one topological sort. Executing on ").ok();
    util::write_bytes_hex_fmt(serial, &[target_addr]).ok();
    writeln!(serial, "...").ok();

    let mut executor =
        PrefixExecutor::<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>::new(prefix, heapless::Vec::new()); // Use calculated size

    for i in 0..sequence_len {
        execute_and_log_command(
            i2c,
            &mut executor,
            serial,
            target_addr,
            &single_sequence.0[i],
            i,
        )?;
    }

    write!(serial, "[explorer] Single sequence execution complete for ").ok();
    util::write_bytes_hex_fmt(serial, &[target_addr]).ok();
    writeln!(serial, ".").ok();

    Ok(())
}
