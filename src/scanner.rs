// scanner.rs
//! Scanner utilities for I2C bus device discovery and analysis.
//!
//! This module provides functions to scan the I2C bus for connected devices,
//! optionally testing with control bytes or initialization command sequences.

use crate::compat::{HalErrorExt, ascii};

pub const I2C_SCAN_ADDR_START: u8 = 0x03;
pub const I2C_SCAN_ADDR_END: u8 = 0x77;

/// Defines the logging level for scanner functions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    /// Log verbose information, including scan progress and detailed errors.
    Verbose,
    /// Log only warnings and errors.
    Normal,
    /// Suppress all logging output.
    Quiet,
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
            log_level: LogLevel,
        ) -> Result<heapless::Vec<u8, 128>, crate::error::ErrorKind>
        where
            I2C: $i2c_trait,
            <I2C as $i2c_trait>::Error: crate::compat::HalErrorExt,
            S: $write_trait,
        {
            let mut found_addrs = heapless::Vec::<u8, 128>::new();
            let mut last_error: Option<crate::error::ErrorKind> = None;
            for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                if let LogLevel::Verbose = log_level {
                    let _ = write!(serial, "[log] Scanning 0x");
                    let _ = ascii::write_byte_hex(serial, addr);
                    let _ = write!(serial, "...");
                }
                match i2c.write(addr, data) {
                    Ok(_) => {
                        found_addrs.push(addr).map_err(|_| {
                            crate::error::ErrorKind::Buffer(crate::error::BufferError::Overflow)
                        })?;
                        if let LogLevel::Verbose = log_level {
                            let _ = writeln!(serial, " Found");
                        }
                    }
                    Err(e) => {
                        let e_kind = e.to_compat(Some(addr));
                        if e_kind == crate::error::ErrorKind::I2c(crate::error::I2cError::Nack)
                        {
                            if let LogLevel::Verbose = log_level {
                                let _ = writeln!(serial, " No response (NACK)");
                            }
                            continue;
                        }
                        if let LogLevel::Verbose = log_level {
                            let _ = write!(serial, "[error] write failed at ");
                            let _ = ascii::write_bytes_hex_prefixed(serial, &[addr]);
                            let _ = writeln!(serial, ": {}", e_kind);
                        }
                        if last_error.is_none() {
                            last_error = Some(e_kind);
                        }
                    }
                }
            }
            if let Some(e) = last_error {
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
            log_level: LogLevel,
        ) -> Result<heapless::Vec<u8, 128>, crate::error::ErrorKind>
        where
            I2C: $i2c_trait,
            <I2C as $i2c_trait>::Error: crate::compat::HalErrorExt,
            S: $write_trait,
        {
            if let LogLevel::Verbose = log_level {
                let _ = writeln!(serial, "[log] Scanning I2C bus...");
            }
            let result = internal_scan(i2c, serial, ctrl_byte, log_level);
            if let Ok(found_addrs) = &result {
                if !found_addrs.is_empty() {
                    match log_level {
                        LogLevel::Verbose => {
                            let _ = writeln!(serial, "[ok] Found devices at:");
                            for addr in found_addrs {
                                let _ = write!(serial, " ");
                                let _ = $crate::compat::ascii::write_bytes_hex_prefixed(serial, &[*addr]);
                                let _ = writeln!(serial, "");
                            }
                            let _ = writeln!(serial);
                        }
                        LogLevel::Normal => {
                            let _ = writeln!(serial, "[ok] Found devices at:");
                            for addr in found_addrs {
                                let _ = writeln!(serial, " 0x{:02X}", addr);
                            }
                            let _ = writeln!(serial);
                        }
                        LogLevel::Quiet => {
                        }
                    }
                }
            }
            if let LogLevel::Verbose = log_level {
                let _ = writeln!(serial, "[info] I2C scan complete.");
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
            init_sequence: &[u8],
            log_level: LogLevel,
        ) -> heapless::Vec<u8, 64>
        where
            I2C: $i2c_trait,
            <I2C as $i2c_trait>::Error: crate::compat::HalErrorExt,
            S: $write_trait,
        {
            if let LogLevel::Verbose = log_level {
                let _ = writeln!(serial, "[scan] Scanning I2C bus with init sequence:");
                for b in init_sequence.iter() {
                    let _ = write!(serial, " ");
                    let _ = $crate::compat::ascii::write_bytes_hex_prefixed(serial, &[*b]);
                    let _ = writeln!(serial, "");
                }
                let _ = writeln!(serial);
            }

            let mut detected_cmds = heapless::Vec::<u8, 64>::new();
            for &cmd in init_sequence.iter() {
                match internal_scan(i2c, serial, &[cmd], log_level) {
                    Ok(found_addrs) => {
                        if !found_addrs.is_empty() {
                            for addr in found_addrs {
                                let _ = write!(serial, "[ok] Found device at ",);
                                let _ = $crate::compat::ascii::write_bytes_hex_prefixed(
                                    serial,
                                    &[addr],
                                );
                                let _ = write!(serial, " responding to ");
                                let _ =
                                    $crate::compat::ascii::write_bytes_hex_prefixed(serial, &[cmd]);
                                let _ = writeln!(serial, "");
                            }
                            if detected_cmds.push(cmd).is_err() {
                                let _ =
                                    writeln!(serial, "[error] Buffer overflow in detected_cmds");
                            }
                        }
                    }
                    Err(e) => {
                        let _ = write!(serial, "[error] scan failed for ");
                        let _ = $crate::compat::ascii::write_bytes_hex_prefixed(serial, &[cmd]);
                        let _ = writeln!(serial, ": {:?}", e);
                    }
                }
            }
            if let LogLevel::Verbose = log_level {
                let _ = writeln!(serial, "[info] I2C scan with init sequence complete.");
            }
            log_differences(serial, init_sequence, &detected_cmds);
            detected_cmds
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
                        let _ = writeln!(
                            serial,
                            "[warn] Missing commands buffer is full, list is truncated."
                        );
                        break;
                    }
                }
            }

            let _ = writeln!(serial, "Expected sequence:");
            for b in expected {
                let _ = write!(serial, " ");
                let _ = ascii::write_bytes_hex_prefixed(serial, &[*b]);
                let _ = writeln!(serial);
            }
            let _ = writeln!(serial);

            let _ = writeln!(serial, "Commands with response:");
            for b in detected {
                let _ = write!(serial, " ");
                let _ = ascii::write_bytes_hex_prefixed(serial, &[*b]);
                let _ = writeln!(serial);
            }
            let _ = writeln!(serial);

            let _ = writeln!(serial, "Commands with no response:");
            for b in &missing_cmds {
                let _ = write!(serial, " ");
                let _ = ascii::write_bytes_hex_prefixed(serial, &[*b]);
                let _ = writeln!(serial);
            }
            let _ = writeln!(serial);
        }
    };
}

/// Runs the I2C explorer with a given initial sequence and logs the results.
///
/// This function first performs an I2C scan with the provided `init_sequence` to identify
/// responsive commands. Then, it uses the `explorer` to find valid command sequences
/// for discovered devices, applying a `prefix` to each command.
///
/// # Type Parameters
///
/// - `I2C`: The I2C interface type that implements `crate::compat::I2cCompat`.
/// - `S`: The serial interface type used for logging, implementing `core::fmt::Write`.
/// - `N`: A const generic for the maximum number of commands.
/// - `BUF_CAP`: A const generic for the command buffer capacity.
///
/// # Parameters
///
/// - `explorer`: An instance of `Explorer` containing the command nodes and their dependencies.
/// - `i2c`: The I2C bus instance.
/// - `serial`: The serial writer for logging.
/// - `init_sequence`: The initial sequence of bytes to test for device responsiveness.
/// - `prefix`: A byte to prepend to every command sent during exploration.
/// - `log_level`: The desired logging level.
///
/// # Example
///
/// ```ignore
/// use dvcdbg::prelude::*;
/// use arduino_hal::I2c;
/// use arduino_hal::hal::port::Port;
/// use arduino_hal::pac::TWI;
/// use heapless::Vec;
/// use core::fmt::Write;
///
/// # struct MyI2c; // Dummy I2c implementation
/// # impl dvcdbg::compat::I2cCompat for MyI2c {
/// #     type Error = dvcdbg::error::ErrorKind;
/// #     fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
/// # }
/// # struct MySerial; // Dummy Serial implementation
/// # impl core::fmt::Write for MySerial {
/// #     fn write_str(&mut self, s: &str) -> core::fmt::Result { Ok(()) }
/// # }
///
/// let mut i2c = /* your I2C instance */;
/// let mut serial = /* your serial instance */;
/// let init_sequence = [0u8; 16]; // Example initial sequence
/// const EXPLORER_CAP: usize = 32;
/// const BUF_CAP: usize = 128;
/// let explorer = Explorer::<EXPLORER_CAP> { sequence: &[] }; // Dummy explorer
///
/// run_explorer::<_, _, EXPLORER_CAP, BUF_CAP>(
///     &explorer,
///     &mut i2c,
///     &mut serial,
///     &init_sequence,
///     0x00, // Example prefix
///     LogLevel::Verbose,
/// ).unwrap();
/// # Ok::<(), dvcdbg::explorer::ExplorerError>(())
/// # }
/// ```
pub fn run_explorer<I2C, S, const N: usize, const BUF_CAP: usize>(
    explorer: &crate::explorer::Explorer<'_, N>,
    i2c: &mut I2C,
    serial: &mut S,
    init_sequence: &[u8],
    prefix: u8,
    log_level: LogLevel,
) -> Result<(), crate::explorer::ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    S: core::fmt::Write,
{
    if let LogLevel::Verbose = log_level {
        let _ = writeln!(serial, "[log] Scanning I2C bus...");
    }
    let successful_seq = crate::scanner::scan_init_sequence(i2c, serial, init_sequence, log_level);
    let _ = writeln!(serial, "[scan] initial sequence scan completed");
    let _ = writeln!(serial, "[log] Start driver safe init");

    // Wrapper for serial interface to implement the Logger trait
        struct SerialLogger<'a, S: core::fmt::Write> {
        writer: &'a mut S,
        buffer: heapless::String<{ crate::explorer::LOG_BUFFER_CAPACITY }>,
        log_level: LogLevel,
    }

    impl<'a, S: core::fmt::Write> SerialLogger<'a, S> {
        fn new(writer: &'a mut S, log_level: LogLevel) -> Self {
            Self {
                writer,
                buffer: heapless::String::new(),
                log_level,
            }
        }
    }

    impl<'a, S: core::fmt::Write> crate::explorer::Logger for SerialLogger<'a, S> {
        fn log_info(&mut self, msg: &str) {
            if self.log_level == LogLevel::Verbose {
                let _ = self.writer.write_str(msg);
            }
        }
        fn log_warning(&mut self, msg: &str) {
            if self.log_level != LogLevel::Quiet {
                let _ = self.writer.write_str(msg);
            }
        }
        fn log_error(&mut self, msg: &str) {
            if self.log_level != LogLevel::Quiet {
                let _ = self.writer.write_str(msg);
            }
        }

        fn log_info_fmt<F>(&mut self, fmt: F)
        where
            F: FnOnce(
                &mut heapless::String<{ crate::explorer::LOG_BUFFER_CAPACITY }>,
            ) -> Result<(), core::fmt::Error>,
        {
            if self.log_level == LogLevel::Verbose {
                self.buffer.clear();
                if fmt(&mut self.buffer).is_ok() {
                    let _ = self.writer.write_str(self.buffer.as_str());
                }
            }
        }

        fn log_error_fmt<F>(&mut self, fmt: F)
        where
            F: FnOnce(
                &mut heapless::String<{ crate::explorer::LOG_BUFFER_CAPACITY }>,
            ) -> Result<(), core::fmt::Error>,
        {
            if self.log_level != LogLevel::Quiet {
                self.buffer.clear();
                if fmt(&mut self.buffer).is_ok() {
                    let _ = self.writer.write_str(self.buffer.as_str());
                }
            }
        }
    }

    // Executor that prepends a prefix and applies an initial sequence once per address.
    struct PrefixExecutor<const BUF_CAP: usize> {
        prefix: u8,
        init_sequence: heapless::Vec<u8, 64>,
        initialized_addrs: [bool; 128], // Use a bitmask for O(1) checks
        buffer: heapless::Vec<u8, BUF_CAP>,
    }

    impl<const BUF_CAP: usize> PrefixExecutor<BUF_CAP> {
        fn new(prefix: u8, init_sequence: heapless::Vec<u8, 64>) -> Self {
            Self {
                prefix,
                init_sequence,
                initialized_addrs: [false; 128],
                buffer: heapless::Vec::new(),
            }
        }
    }

    impl<I2C, const BUF_CAP: usize> crate::explorer::CmdExecutor<I2C> for PrefixExecutor<BUF_CAP>
    where
        I2C: crate::compat::I2cCompat,
        <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    {
        fn exec(
            &mut self,
            i2c: &mut I2C,
            addr: u8,
            cmd: &[u8],
        ) -> Result<(), crate::explorer::ExecutorError> {
            let addr_idx = addr as usize;

            // Check if the address has already been initialized (O(1) check)
            if !self.initialized_addrs[addr_idx] {
                // First, send the init_sequence with the prefix, one command at a time.
                for &c in self.init_sequence.iter() {
                    let command = [self.prefix, c];
                    i2c.write(addr, &command).map_err(|e| {
                        crate::explorer::ExecutorError::I2cError(e.to_compat(Some(addr)))
                    })?;
                }
                // Mark this address as initialized
                self.initialized_addrs[addr_idx] = true;
            }

            // Then, send the regular command. Reuse the buffer.
            self.buffer.clear();
            self.buffer
                .push(self.prefix)
                .map_err(|_| crate::explorer::ExecutorError::BufferOverflow)?;
            self.buffer
                .extend_from_slice(cmd)
                .map_err(|_| crate::explorer::ExecutorError::BufferOverflow)?;

            i2c.write(addr, &self.buffer)
                .map_err(|e| crate::explorer::ExecutorError::I2cError(e.to_compat(Some(addr))))
        }
    }

    let mut executor = PrefixExecutor::<BUF_CAP>::new(prefix, successful_seq);
    let mut serial_logger = SerialLogger::new(serial, log_level);

    for addr in explorer
        .explore(i2c, &mut executor, &mut serial_logger)?
        .found_addrs
        .iter()
    {
        let _ = write!(serial, "[driver] Found device at ");
        let _ = ascii::write_bytes_hex_prefixed(serial, &[*addr]);
        let _ = writeln!(serial);
    }

    Ok(())
}

define_scanner!(
    crate::compat::I2cCompat,
    crate::compat::HalErrorExt,
    core::fmt::Write
);
