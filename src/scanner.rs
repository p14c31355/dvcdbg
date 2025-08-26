//! Scanner utilities for I2C bus device discovery and analysis.
//!
//! This module provides functions to scan the I2C bus for connected devices,
//! optionally testing with control bytes or initialization command sequences.

use heapless::Vec;

pub const I2C_SCAN_ADDR_START: u8 = 0x03;
pub const I2C_SCAN_ADDR_END: u8 = 0x77;

#[derive(Clone, Copy)]
pub enum LogLevel {
    Quiet,
    Verbose,
}

#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
pub mod ehal_0_2 {
    use crate::define_scanner;
    define_scanner!(crate::compat::I2cCompat);
}

#[cfg(feature = "ehal_1_0")]
pub mod ehal_1_0 {
    use crate::define_scanner;
    define_scanner!(crate::compat::I2cCompat);
}

#[cfg(feature = "ehal_1_0")]
pub use self::ehal_1_0::{scan_i2c, scan_i2c_with_ctrl, scan_init_sequence};

#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
pub use self::ehal_0_2::{scan_i2c, scan_i2c_with_ctrl, scan_init_sequence};

#[macro_export]
macro_rules! define_scanner {
    ($i2c_trait:path) => {
        use heapless::Vec;
        use $crate::error::{ErrorKind, I2cError};
        use $crate::compat::HalErrorExt;
        /// Scan the I2C bus for connected devices (addresses `0x03` to `0x77`).
        ///
        /// This function probes each possible I2C device address by attempting to
        /// write an empty buffer (`[]`). Devices that acknowledge are reported
        /// through the provided logger.
        ///
        /// # Arguments
        ///
        /// * `i2c` - Mutable reference to an I2C interface implementing the `write` method.
        /// * `serial` - Mutable reference to a type implementing [`core::fmt::Write`].
        /// * `log_level` - Controls the verbosity of the log output.
        /// # Example
        ///
        /// ```ignore
        /// use embedded_hal::i2c::I2c;
        /// use dvcdbg::scanner::scan_i2c;
        ///
        /// let mut i2c = /* your i2c interface */;
        /// let mut serial = /* your type implementing core::fmt::Write */;
        ///
        /// scan_i2c(&mut i2c, &mut serial, Quiet);
        /// ```
        pub fn scan_i2c<I2C, W>(i2c: &mut I2C, serial: &mut W, log_level: $crate::scanner::LogLevel)
        where
            I2C: $i2c_trait,
            W: core::fmt::Write,
            <I2C as $i2c_trait>::Error: HalErrorExt,
        {
            let _ = writeln!(serial, "[scan] Scanning I2C bus...");
            if let Ok(found_addrs) = internal_scan(i2c, serial, &[], log_level) {
                if !found_addrs.is_empty() {
                    let _ = writeln!(serial, "[ok] Found devices at:");
                    for addr in &found_addrs {
                        let _ = writeln!(serial, " 0x{:02X}", addr);
                    }
                    let _ = writeln!(serial);
                }
            }
            let _ = writeln!(serial, "[info] I2C scan complete.");
        }

        /// Scan the I2C bus for devices by sending specified control bytes.
        ///
        /// This variant allows specifying control bytes (e.g., `0x00`) to send
        /// alongside the probe. Devices that acknowledge the transmission are
        /// reported.
        ///
        /// # Arguments
        ///
        /// * `i2c` - Mutable reference to an I2C interface implementing the `write` method.
        /// * `serial` - Mutable reference to a type implementing [`core::fmt::Write`].
        /// * `control_bytes` - Slice of bytes to send when probing each device.
        /// * `log_level` - Controls the verbosity of the log output.
        ///
        /// # Example
        ///
        /// ```ignore
        /// use embedded_hal::i2c::I2c;
        /// use dvcdbg::scanner::scan_i2c_with_ctrl;
        ///
        /// let mut i2c = /* your i2c interface */;
        /// let mut serial = /* your type implementing core::fmt::Write */;
        ///
        /// scan_i2c_with_ctrl(&mut i2c, &mut serial, &[0x00], Quiet);
        /// ```
        pub fn scan_i2c_with_ctrl<I2C, W>(
            i2c: &mut I2C,
            serial: &mut W,
            control_bytes: &[u8],
            log_level: $crate::scanner::LogLevel,
        ) where
            I2C: $i2c_trait,
            W: core::fmt::Write,
            <I2C as $i2c_trait>::Error: HalErrorExt,
        {
            let _ = writeln!(serial, "[scan] Scanning I2C bus with control bytes:");
            for b in control_bytes {
                let _ = writeln!(serial, " 0x{:02X}", b);
            }
            let _ = writeln!(serial);
            if let Ok(found_addrs) = internal_scan(i2c, serial, control_bytes, log_level) {
                if !found_addrs.is_empty() {
                    let _ = writeln!(serial, "[ok] Found devices at:");
                    for addr in &found_addrs {
                        let _ = writeln!(serial, " 0x{:02X}", addr);
                    }
                    let _ = writeln!(serial);
                }
            }
            let _ = writeln!(serial, "[info] I2C scan complete.");
        }

        /// Scan the I2C bus using an initialization sequence of commands.
        ///
        /// Each command in the sequence is transmitted to all possible device
        /// addresses using the control byte `0x00`. The function records which
        /// commands receive responses and highlights any **differences** between
        /// the expected and observed responses.
        ///
        /// This is useful for verifying whether a device supports the expected
        /// initialization commands (e.g., when testing display controllers).
        ///
        /// # Arguments
        ///
        /// * `i2c` - Mutable reference to an I2C interface implementing the `write` method.
        /// * `serial` - Mutable reference to a type implementing [`core::fmt::Write`].
        /// * `init_sequence` - Slice of initialization commands to test.
        /// * `log_level` - Controls the verbosity of the log output.
        ///
        /// # Example
        ///
        /// ```ignore
        /// use embedded_hal::i2c::I2c;
        /// use dvcdbg::scanner::scan_init_sequence;
        ///
        /// let mut i2c = /* your i2c interface */;
        /// let mut serial = /* your type implementing core::fmt::Write */;
        ///
        /// let init_sequence: [u8; 3] = [0xAE, 0xA1, 0xAF]; // example init cmds
        /// scan_init_sequence(&mut i2c, &mut serial, &init_sequence, Quiet);
        /// ```
        pub fn scan_init_sequence<I2C, W>(
            i2c: &mut I2C,
            serial: &mut W,
            init_sequence: &[u8],
            log_level: $crate::scanner::LogLevel,
        ) -> Vec<u8, 64>
        where
            I2C: $i2c_trait,
            W: core::fmt::Write,
            <I2C as $i2c_trait>::Error: HalErrorExt,
        {
            let _ = writeln!(serial, "[scan] Scanning I2C bus with init sequence:");
            for b in init_sequence.iter() {
                let _ = writeln!(serial, " 0x{b:02X}");
            }
            let _ = writeln!(serial);

            let mut detected_cmds: Vec<u8, 64> = Vec::new();
            for &cmd in init_sequence.iter() {
                let value = internal_scan(i2c, serial, &[0x00, cmd], log_level.clone());
                match value {
                    Ok(found_addrs) => {
                        if !found_addrs.is_empty() {
                            for addr in found_addrs {
                                let _ = writeln!(serial,
                                    "[ok] Found device at 0x{:02X} responding to 0x{:02X}",
                                    addr, cmd
                                );
                            }
                            if detected_cmds.push(cmd).is_err() {
                                let _ = writeln!(serial,
                                    "[warn] Detected commands buffer is full, results may be incomplete!"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        let _ = writeln!(serial, "[error] scan failed for 0x{:02X}: {:?}", cmd, e);
                    }
                }
            }

            super::log_differences(serial, init_sequence, &detected_cmds);
            let _ = writeln!(serial, "[info] I2C scan with init sequence complete.");
            detected_cmds
        }

        fn internal_scan<I2C, W>(
            i2c: &mut I2C,
            serial: &mut W,
            data: &[u8],
            log_level: $crate::scanner::LogLevel,
        ) -> Result<Vec<u8, 128>, ErrorKind>
        where
            I2C: $i2c_trait,
            W: core::fmt::Write,
            <I2C as $i2c_trait>::Error: HalErrorExt,
        {
            let mut found_devices: Vec<u8, 128> = Vec::new();
            let mut last_error: Option<ErrorKind> = None;

            for addr in super::I2C_SCAN_ADDR_START..=super::I2C_SCAN_ADDR_END {
                match i2c.write(addr, data) {
                    Ok(_) => { let _ = found_devices.push(addr); }
                    Err(e) => {
                        let e_kind = e.to_compat(Some(addr));
                        if e_kind == ErrorKind::I2c(I2cError::Nack) {
                            continue;
                        }
                        if let $crate::scanner::LogLevel::Verbose = log_level {
                            let _ = writeln!(serial, "[error] write failed at 0x{:02X}: {}", addr, e_kind);
                        }
if last_error.is_none() { last_error = Some(e_kind); }
                    }
                }
            }

            if found_devices.is_empty() {
                if let Some(e) = last_error {
                    Err(e)
                } else {
                    Ok(found_devices)
                }
            } else {
                Ok(found_devices)
            }
        }
    }
}

fn log_differences<W: core::fmt::Write>(serial: &mut W, expected: &[u8], detected: &Vec<u8, 64>) {
    let _ = writeln!(serial, "Expected sequence:");
    for b in expected {
        let _ = writeln!(serial, " 0x{b:02X}");
    }
    let _ = writeln!(serial);

    let _ = writeln!(serial, "Commands with response:");
    for b in detected {
        let _ = writeln!(serial, " 0x{b:02X}");
    }
    let _ = writeln!(serial);

    let mut sorted = detected.clone();
    sorted.sort_unstable();
    let mut missing_cmds: Vec<u8, 64> = Vec::new();
    for cmd in expected
        .iter()
        .copied()
        .filter(|c| sorted.binary_search(c).is_err())
    {
        if missing_cmds.push(cmd).is_err() {
            let _ = writeln!(
                serial,
                "[warn] Missing commands buffer is full, list is truncated."
            );
            break;
        }
    }

    let _ = writeln!(serial, "Commands with no response:");
    for b in &missing_cmds {
        let _ = writeln!(serial, " 0x{b:02X}");
    }
    let _ = writeln!(serial);
}

/// Runs an I2C initialization exploration sequence using a provided `Explorer`.
///
/// This function scans the I2C bus for devices and attempts to apply
/// a safe driver initialization sequence. It logs progress and errors
/// to a serial interface implementing `core::fmt::Write`.
///
/// # Type Parameters
///
/// - `I2C`: The I2C interface type that implements `crate::compat::I2cCompat`.
/// - `S`: The serial interface type used for logging, implementing `core::fmt::Write`.
///
/// # Parameters
///
/// - `explorer`: A reference to an `Explorer` that defines the exploration sequence.
/// - `i2c`: A mutable reference to the I2C bus to communicate with devices.
/// - `serial`: A mutable reference to a serial logger for debug output.
/// - `init_sequence`: A slice of bytes representing the initial I2C command sequence.
///   This sequence will be scanned and sent to each device before running the explorer commands.
/// - `prefix`: A byte to prepend to each command during execution.
/// - `log_level`: A `LogLevel` controlling the verbosity of logging.
///
/// # Returns
///
/// Returns `Ok(())` if the explorer successfully applied the driver-safe init sequence,
/// or `Err(ExplorerError)` if the exploration failed. Errors are also logged to `serial`.
///
/// # Example
///
/// ```ignore
/// # use dvcdbg::prelude::*;
/// # use dvcdbg::explorer::ExplorerError;
/// # use dvcdbg::scanner::LogLevel;
/// # fn main() -> Result<(), ExplorerError> {
/// let mut i2c = /* your I2C instance */;
/// let mut serial = /* your serial instance */;
/// let init_sequence = [0u8; 16]; // Example initial sequence
/// let explorer = Explorer { sequence: &[] }; // Dummy explorer
///
/// run_explorer(
///     &explorer,
///     &mut i2c,
///     &mut serial,
///     &init_sequence,
///     0x3C, // Example prefix
///     LogLevel::Verbose,
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn run_explorer<I2C, S, const N: usize>(
    explorer: &crate::explorer::Explorer<'_, N>,
    i2c: &mut I2C,
    serial: &mut S,
    init_sequence: &[u8],
    prefix: u8,
    log_level: LogLevel,
) -> Result<(), crate::explorer::ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    S: core::fmt::Write,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
{
    let _ = writeln!(serial, "[log] Scanning I2C bus...");
    let successful_seq = crate::scanner::scan_init_sequence(i2c, serial, init_sequence, log_level);
    let _ = writeln!(serial, "[scan] initial sequence scan completed");
    let _ = writeln!(serial, "[log] Start driver safe init");

    explorer
        .explore(
            i2c,
            serial,
            &mut PrefixExecutor::new(prefix, successful_seq),
        )
        .map(|()| {
            let _ = writeln!(serial, "[driver] init sequence applied");
        })
        .map_err(|e| {
            let _ = writeln!(serial, "[error] explorer failed: {e:?}");
            e
        })
}

/// Executor that prepends a prefix to each command and applies an initial sequence.
///
/// `PrefixExecutor` is used internally by `run_explorer` to ensure that each I2C
/// command is sent with a fixed prefix and that the initial scanned sequence is
/// executed for every device before the main exploration commands.
pub struct PrefixExecutor {
    prefix: u8,
    init_sequence: heapless::Vec<u8, 64>,
    buffer: heapless::Vec<u8, 64>,
}

impl PrefixExecutor {
    /// Creates a new `PrefixExecutor` with the given prefix and initial sequence.
    ///
    /// # Parameters
    ///
    /// - `prefix`: Byte to prepend to every command.
    /// - `init_sequence`: Initial command sequence to run before explorer commands.
    pub fn new(prefix: u8, init_sequence: heapless::Vec<u8, 64>) -> Self {
        Self {
            prefix,
            init_sequence,
            buffer: heapless::Vec::new(),
        }
    }
}

impl<I2C> crate::explorer::CmdExecutor<I2C> for PrefixExecutor
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
{
    /// Executes a command on the I2C device at `addr`.
    ///
    /// This method first sends the initial sequence with the prefix to the device,
    /// then sends the provided command with the prefix.
    ///
    /// Returns `true` if the command was successfully written, or `false` on failure.
    fn exec(&mut self, i2c: &mut I2C, addr: u8, cmd: &[u8]) -> Result<(), ()> {
        // Init sequence before exploring
        for &c in self.init_sequence.iter() {
            let command = [self.prefix, c];
            if i2c.write(addr, &command).is_err() {
                return Err(());
            }
        }

        // Run the explorer command
        self.buffer.clear();
        if self.buffer.push(self.prefix).is_err() || self.buffer.extend_from_slice(cmd).is_err() {
            return Err(());
        }

        i2c.write(addr, &self.buffer).is_ok().then_some(()).ok_or(())
    }
}