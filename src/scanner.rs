// scanner.rs
//! Scanner utilities for I2C bus device discovery and analysis.
//!
//! This module provides functions to scan the I2C bus for connected devices,
//! optionally testing with control bytes or initialization command sequences.

use crate::{
    compat::{HalErrorExt, ascii},
    logger::Logger,
    prelude::CmdExecutor,
};
use core::fmt::Write;

pub const I2C_SCAN_ADDR_START: u8 = 0x03;
pub const I2C_SCAN_ADDR_END: u8 = 0x77;

/// A command executor that prepends a prefix to each command.
pub struct PrefixExecutor<const BUF_CAP: usize> {
    prefix: u8,
    init_sequence: heapless::Vec<u8, 64>,
    initialized_addrs: [bool; 128],
    buffer: heapless::Vec<u8, BUF_CAP>,
}

impl<const BUF_CAP: usize> PrefixExecutor<BUF_CAP> {
    pub fn new(prefix: u8, init_sequence: heapless::Vec<u8, 64>) -> Self {
        Self {
            prefix,
            init_sequence,
            initialized_addrs: [false; 128],
            buffer: heapless::Vec::new(),
        }
    }
}

impl<I2C, const BUF_CAP: usize> crate::explorer::CmdExecutor<I2C, BUF_CAP>
    for PrefixExecutor<BUF_CAP>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
{
    fn exec<S>(
        &mut self,
        i2c: &mut I2C,
        addr: u8,
        cmd: &[u8],
        logger: &mut S,
    ) -> Result<(), crate::explorer::ExecutorError>
    where
        S: core::fmt::Write + crate::logger::Logger<BUF_CAP>,
    {
        fn short_delay() {
            for _ in 0..8_000 {
                core::hint::spin_loop();
            }
        }

        let addr_idx = addr as usize;

        if !self.initialized_addrs[addr_idx] && !self.init_sequence.is_empty() {
            logger
                .log_info_fmt(|buf| write!(buf, "[Info] I2C initializing for 0x{addr:02X}...\r\n"));

            for &c in self.init_sequence.iter() {
                let command = [self.prefix, c];
                let mut ok = false;

                for _attempt in 0..10 {
                    match i2c.write(addr, &command) {
                        Ok(_) => {
                            ok = true;
                            break;
                        }
                        Err(e) => {
                            let compat_err = e.to_compat(Some(addr));
                            logger.log_error_fmt(|buf| {
                                write!(buf, "[I2C retry error] {compat_err:?}\r\n")
                            });
                            short_delay();
                        }
                    }
                }

                if !ok {
                    return Err(crate::explorer::ExecutorError::I2cError(
                        crate::error::ErrorKind::I2c(crate::error::I2cError::Nack),
                    ));
                }
                short_delay();
            }

            self.initialized_addrs[addr_idx] = true;
            logger.log_info_fmt(|buf| write!(buf, "[Info] I2C initialized for 0x{addr:02X}\r\n"));
        }

        self.buffer.clear();
        self.buffer
            .push(self.prefix)
            .map_err(|_| crate::explorer::ExecutorError::BufferOverflow)?;
        self.buffer
            .extend_from_slice(cmd)
            .map_err(|_| crate::explorer::ExecutorError::BufferOverflow)?;

        for _ in 0..10 {
            match i2c.write(addr, &self.buffer) {
                Ok(_) => {
                    short_delay();
                    return Ok(());
                }
                Err(e) => {
                    let compat_err = e.to_compat(Some(addr));
                    logger.log_error_fmt(|buf| write!(buf, "[I2C retry error] {compat_err:?}\r\n"));
                    short_delay();
                }
            }
        }

        Err(crate::explorer::ExecutorError::I2cError(
            crate::error::ErrorKind::I2c(crate::error::I2cError::Nack),
        ))
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
        /// Internal function to perform an I2C scan.
        ///
        /// Attempts to write a single byte to each address in the I2C range.
        ///
        /// # Parameters
        ///
        /// - `i2c`: The I2C bus instance.
        /// - `serial`: The serial writer for logging.
        /// - `control_bytes`: An array of control bytes to try for each address.
        /// - `log_level`: The desired logging level.
        ///
        /// # Returns
        ///
        /// A `Result` containing a `Vec` of found addresses on success, or an `ErrorKind` on failure.
        fn internal_scan<I2C, S>(
            i2c: &mut I2C,
            serial: &mut S,
            data: &[u8],
            log_level: $crate::logger::LogLevel,
        ) -> Result<heapless::Vec<u8, 128>, $crate::error::ErrorKind>
        where
            I2C: $i2c_trait,
            <I2C as $i2c_trait>::Error: $crate::compat::HalErrorExt,
            S: $write_trait,
        {
            let mut found_addrs = heapless::Vec::<u8, 128>::new();
            let mut last_error: Option<crate::error::ErrorKind> = None;
            for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                if let $crate::logger::LogLevel::Verbose = log_level {
                    write!(serial, "[log] Scanning 0x").ok();
                    ascii::write_bytes_hex_fmt(serial, &[addr]).ok();
                    write!(serial, "...").ok();
                }
                match i2c.write(addr, data) {
                    Ok(_) => {
                        found_addrs.push(addr).map_err(|_| {
                            $crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
                        })?;
                        if let $crate::logger::LogLevel::Verbose = log_level {
                            writeln!(serial, " Found").ok();
                        }
                    }
                    Err(e) => {
                        let e_kind = e.to_compat(Some(addr));
                        if e_kind == $crate::error::ErrorKind::I2c(crate::error::I2cError::Nack) {
                            if let $crate::logger::LogLevel::Verbose = log_level {
                                writeln!(serial, " No response (NACK)").ok();
                            }
                            continue;
                        }
                        if let $crate::logger::LogLevel::Verbose = log_level {
                            write!(serial, "[error] write failed at ").ok();
                            ascii::write_bytes_hex_fmt(serial, &[addr]).ok();
                            writeln!(serial, ": {}", e_kind).ok();
                        }
                        if last_error.is_none() {
                            last_error = Some(e_kind);
                        }
                    }
                }
            }
            if !found_addrs.is_empty() {
                Ok(found_addrs)
            } else if let Some(e) = last_error {
                Err(e)
            } else {
                Ok(found_addrs)
            }
        }

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
            log_level: $crate::logger::LogLevel,
        ) -> Result<heapless::Vec<u8, 128>, $crate::error::ErrorKind>
        where
            I2C: $i2c_trait,
            <I2C as $i2c_trait>::Error: $crate::compat::HalErrorExt,
            S: $write_trait,
        {
            if let $crate::logger::LogLevel::Verbose = log_level {
                writeln!(serial, "[log] Scanning I2C bus...").ok();
            }
            let result = internal_scan(i2c, serial, ctrl_byte, log_level);
            if let Ok(found_addrs) = &result {
                if !found_addrs.is_empty() {
                    match log_level {
                        $crate::logger::LogLevel::Verbose => {
                            writeln!(serial, "[ok] Found devices at:").ok();
                            for addr in found_addrs {
                                write!(serial, " ").ok();
                                $crate::compat::ascii::write_bytes_hex_fmt(serial, &[*addr]).ok();
                                writeln!(serial, "").ok();
                            }
                            writeln!(serial).ok();
                        }
                        $crate::logger::LogLevel::Normal => {
                            for addr in found_addrs {
                                write!(serial, " ").ok();
                                $crate::compat::ascii::write_bytes_hex_fmt(serial, &[*addr]).ok();
                                writeln!(serial, "").ok();
                            }
                            writeln!(serial).ok();
                        }
                        $crate::logger::LogLevel::Quiet => {}
                    }
                }
            }
            if let $crate::logger::LogLevel::Verbose = log_level {
                writeln!(serial, "[info] I2C scan complete.").ok();
            }
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
            log_level: $crate::logger::LogLevel,
        ) -> Result<heapless::Vec<u8, 64>, $crate::error::ErrorKind>
        where
            I2C: $i2c_trait,
            <I2C as $i2c_trait>::Error: $crate::compat::HalErrorExt,
            S: $write_trait,
        {
            if let $crate::logger::LogLevel::Verbose = log_level {
                writeln!(serial, "[scan] Scanning I2C bus with init sequence:").ok();
                for chunk in init_sequence.chunks(16) {
                    write!(serial, " ").ok();
                    $crate::compat::ascii::write_bytes_hex_fmt(serial, chunk).ok();
                    writeln!(serial).ok();
                }
            }
            let initial_found_addrs =
                crate::scanner::scan_i2c(i2c, serial, &[ctrl_byte], log_level)?;

            let mut detected_cmds = heapless::Vec::<u8, 64>::new();
            let mut last_error: Option<$crate::error::ErrorKind> = None;

            for &seq_cmd in init_sequence.iter() {
                match internal_scan(i2c, serial, &[ctrl_byte, seq_cmd], log_level) {
                    Ok(responded_addrs_for_cmd) => {
                        let mut cmd_responded_by_initial_device = false;
                        for &addr in responded_addrs_for_cmd.iter() {
                            if initial_found_addrs.contains(&addr) {
                                if let $crate::logger::LogLevel::Verbose = log_level {
                                    write!(serial, "[ok] Found device at ").ok();
                                    $crate::compat::ascii::write_bytes_hex_fmt(serial, &[addr])
                                        .ok();
                                    write!(serial, " responding to ").ok();
                                    $crate::compat::ascii::write_bytes_hex_fmt(serial, &[seq_cmd])
                                        .ok();
                                    writeln!(serial, "").ok();
                                }
                                cmd_responded_by_initial_device = true;
                            }
                        }
                        if cmd_responded_by_initial_device {
                            if detected_cmds.push(seq_cmd).is_err() {
                                writeln!(serial, "[error] Buffer overflow in detected_cmds").ok();
                            }
                        }
                    }
                    Err(e) => {
                        write!(serial, "[error] scan failed for command 0x").ok();
                        $crate::compat::ascii::write_bytes_hex_fmt(serial, &[seq_cmd]).ok();
                        writeln!(serial, ": {:?}", e).ok();
                        if last_error.is_none() {
                            last_error = Some(e);
                        }
                    }
                }
            }
            if let $crate::logger::LogLevel::Verbose = log_level {
                writeln!(serial, "[info] I2C scan with init sequence complete.").ok();
            }
            log_differences(serial, init_sequence, &detected_cmds);
            if let Some(e) = last_error {
                Err(e)
            } else {
                Ok(detected_cmds)
            }
        }

        fn log_differences<W: core::fmt::Write>(
            serial: &mut W,
            expected: &[u8],
            detected: &heapless::Vec<u8, 64>,
        ) {
            let mut missing_cmds = heapless::Vec::<u8, 64>::new();
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

pub fn run_explorer<I2C, S, const N: usize, const BUF_CAP: usize>(
    explorer: &crate::explorer::Explorer<'_, N>,
    i2c: &mut I2C,
    serial: &mut S,
    init_sequence: &[u8],
    prefix: u8,
    log_level: crate::logger::LogLevel,
) -> Result<(), crate::explorer::ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let mut serial_logger = crate::logger::SerialLogger::new(serial, log_level);

    serial_logger.log_info_fmt(|buf| write!(buf, "[log] Initial I2C bus scan..."));

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
                write!(
                    buf,
                    "[error] Initial sequence scan failed: {e:?}. Aborting explorer."
                )
            });
            return Err(crate::explorer::ExplorerError::ExecutionFailed);
        }
    };
    serial_logger.log_info_fmt(|buf| write!(buf, "[scan] initial sequence scan completed"));
    serial_logger.log_info_fmt(|buf| write!(buf, "[log] Start driver safe init"));

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
    explorer: &crate::explorer::Explorer<'_, N>,
    i2c: &mut I2C,
    executor: &mut E,
    serial: &mut S,
    prefix: u8,
    log_level: crate::logger::LogLevel,
) -> Result<(), crate::explorer::ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    E: crate::explorer::CmdExecutor<I2C, BUF_CAP>,
    S: core::fmt::Write + crate::logger::Logger<BUF_CAP>,
{
    let mut serial_logger = crate::logger::SerialLogger::new(serial, log_level);

    let mut failed_nodes = [false; N];
    let mut solved_addrs = [false; 128];
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
                            write!(buf, "[error] Failed to generate a new topological sort. Aborting.")
                        });
                        return Err(e);
                    }
                }
            };

        for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
            if solved_addrs[addr as usize] {
                continue;
            }

            let mut all_ok = true;
            for i in 0..explorer.sequence.len() {
                let cmd_bytes = &sequence_bytes[i][..sequence_len[i]];
                match executor.exec(i2c, addr, cmd_bytes, &mut serial_logger) {
                    Ok(_) => {}
                    Err(_) => {
                        failed_nodes[i] = true;
                        all_ok = false;
                        break;
                    }
                }
            }

            if all_ok {
                solved_addrs[addr as usize] = true;
                commands_found += explorer.sequence.len();
                serial_logger.log_info_fmt(|buf| {
                    write!(buf, "[ok] Device at 0x{:02X} successfully initialized.", addr)
                });
            }
        }

        let all_nodes_visited = failed_nodes.iter().all(|&x| x) || solved_addrs.iter().all(|&x| x);
        if all_nodes_visited {
            serial_logger.log_info("[explorer] Exploration complete.");
            return Ok(());
        }
    }
}

pub fn run_single_sequence_explorer<I2C, S, const N: usize, const BUF_CAP: usize, const MAX_CMD_LEN: usize>(
    explorer: &crate::explorer::Explorer<'_, N>,
    i2c: &mut I2C,
    serial: &mut S,
    target_addr: u8,
    prefix: u8,
    log_level: crate::logger::LogLevel,
) -> Result<(), crate::explorer::ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    let mut serial_logger = crate::logger::SerialLogger::new(serial, log_level);
    serial_logger.log_info_fmt(|buf| {
        write!(
            buf,
            "[explorer] Attempting to get one topological sort...\r\n"
        )?;
        Ok(())
    });

    let single_sequence = explorer.get_one_topological_sort_buf::<MAX_CMD_LEN>(&mut serial_logger, &[false; N])?;
    serial_logger.log_info_fmt(|buf| write!(buf, "Before sort:\r\n"));
    for (idx, node) in explorer.sequence.iter().enumerate() {
        serial_logger.log_info_fmt(|buf| write!(buf, "Node {idx} deps: {:?}\r\n", node.deps));
    }

    let sequence_len = explorer.sequence.len();

    serial_logger.log_info_fmt(|buf| {
        write!(
            buf,
            "[explorer] Obtained one topological sort. Executing on 0x{target_addr:02X}...\r\n"
        )?;
        Ok(())
    });

    for node_idx in 0..explorer.sequence.len() {
        writeln!(serial_logger, "Checking node {node_idx}").ok();
    }

    let mut executor = PrefixExecutor::<BUF_CAP>::new(prefix, heapless::Vec::new());

    for i in 0..sequence_len {
        serial_logger.log_info_fmt(|buf| {
            write!(
                buf,
                "[explorer] Sending node {} bytes: {:02X?} ... ",
                i, single_sequence.0[i]
            )?;
            Ok(())
        });
        match executor.exec(i2c, target_addr, &single_sequence.0[i], &mut serial_logger) {
            Ok(_) => {
                serial_logger.log_info_fmt(|buf| {
                    write!(buf, "OK\r\n")?;
                    Ok(())
                });
            }
            Err(e) => {
                serial_logger.log_error_fmt(|buf| {
                    write!(buf, "FAILED: {e:?}\r\n")?; // `e` is now in scope
                    Ok(())
                });
                return Err(e.into()); // Convert ExecutorError to ExplorerError and return
            }
        };
    }

    serial_logger.log_info_fmt(|buf| {
        write!(
            buf,
            "[explorer] Single sequence execution complete for 0x{target_addr:02X}.\r\n"
        )?;
        Ok(())
    });

    Ok(())
}

define_scanner!(
    crate::compat::I2cCompat,
    crate::compat::HalErrorExt,
    core::fmt::Write
);
