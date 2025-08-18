//! Scanner utilities for I2C bus device discovery and analysis.
//!
//! This module provides functions to scan the I2C bus for connected devices,
//! optionally testing with control bytes or initialization command sequences,
//! with detailed logging support.

use crate::log;
use crate::logger::Logger;
use heapless::Vec;

const I2C_SCAN_ADDR_START: u8 = 0x03;
const I2C_SCAN_ADDR_END: u8 = 0x77;

// -----------------------------------------------------------------------------
//  Version branching
// -----------------------------------------------------------------------------

#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
impl<I2C> I2cCompat for I2C
where
    I2C: embedded_hal_0_2::blocking::i2c::Write,
    <I2C as embedded_hal_0_2::blocking::i2c::Write>::Error: Debug + Copy,
{
    type Error = <I2C as embedded_hal_0_2::blocking::i2c::Write>::Error;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        embedded_hal_0_2::blocking::i2c::Write::write(self, addr, bytes)
    }
}

#[cfg(feature = "ehal_1_0")]
impl<I2C> I2cCompat for I2C
where
    I2C: embedded_hal_1::i2c::I2c,
    I2C::Error: Into<embedded_hal_1::i2c::ErrorKind> + Debug + Copy,
{
    type Error = I2C::Error;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        embedded_hal_1::i2c::I2c::write(self, addr, bytes)
    }
}

#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
pub mod ehal_0_2 {
    use crate::define_scanner;
    use crate::log;
    define_scanner!(crate::scanner::I2cCompat, crate::logger::Logger, embedded_hal_0_2::blocking::i2c::Error);
}

#[cfg(feature = "ehal_1_0")]
pub mod ehal_1_0 {
    use crate::define_scanner;
    use crate::log;
    define_scanner!(crate::scanner::I2cCompat, crate::logger::Logger, embedded_hal_1::i2c::Error);
}

#[cfg(feature = "ehal_1_0")]
pub use self::ehal_1_0::{scan_i2c, scan_i2c_with_ctrl, scan_init_sequence};

#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
pub use self::ehal_0_2::{scan_i2c, scan_i2c_with_ctrl, scan_init_sequence};

#[macro_export]
macro_rules! define_scanner {
    ($i2c_trait:path, $logger_trait:path, $($error_bound:tt)*) => {
        /// Scan the I2C bus for connected devices (addresses `0x03` to `0x77`).
        ///
        /// This function probes each possible I2C device address by attempting to
        /// write an empty buffer (`[]`). Devices that acknowledge are reported
        /// through the provided logger.
        ///
        /// # Arguments
        ///
        /// * `i2c` - Mutable reference to an I2C interface implementing the `write` method.
        /// * `logger` - Mutable reference to a logger implementing the [`Logger`] trait.
        ///
        /// # Example
        ///
        /// ```ignore
        /// use embedded_hal::i2c::I2c;
        /// use dvcdbg::logger::SerialLogger;
        /// use dvcdbg::scanner::scan_i2c;
        ///
        /// let mut i2c = /* your i2c interface */;
        /// let mut logger = SerialLogger::new(/* serial */);
        ///
        /// scan_i2c(&mut i2c, &mut logger);
        /// ```
        pub fn scan_i2c<I2C, L>(i2c: &mut I2C, logger: &mut L)
        where
            I2C: $i2c_trait,
            <I2C as $i2c_trait>::Error: $($error_bound)*,
            L: $logger_trait,
        {
            log!(logger, "[scan] Scanning I2C bus...");
            match internal_scan(i2c, logger, &[]) {
                Ok(found_addrs) => {
                    for addr in found_addrs {
                        log!(logger, "[ok] Found device at 0x{:02X}", addr);
                    }
                }
                Err(e) => log!(logger, "[error] I2C scan failed: {:?}", e),
            }
            log!(logger, "[info] I2C scan complete.");
        }

        /// Scan the I2C bus for devices by sending specified control bytes.
        ///
        /// This variant allows specifying control bytes (e.g., `0x00`) to send
        /// alongside the probe. Devices that acknowledge the transmission are
        /// reported via the logger.
        ///
        /// # Arguments
        ///
        /// * `i2c` - Mutable reference to an I2C interface implementing the `write` method.
        /// * `logger` - Mutable reference to a logger implementing the [`Logger`] trait.
        /// * `control_bytes` - Slice of bytes to send when probing each device.
        ///
        /// # Example
        ///
        /// ```ignore
        /// use embedded_hal::i2c::I2c;
        /// use dvcdbg::logger::SerialLogger;
        /// use dvcdbg::scanner::scan_i2c_with_ctrl;
        ///
        /// let mut i2c = /* your i2c interface */;
        /// let mut logger = SerialLogger::new(/* serial */);
        ///
        /// scan_i2c_with_ctrl(&mut i2c, &mut logger, &[0x00]);
        /// ```
        pub fn scan_i2c_with_ctrl<I2C, L>(
            i2c: &mut I2C,
            logger: &mut L,
            control_bytes: &[u8],
        ) where
            I2C: $i2c_trait,
            <I2C as $i2c_trait>::Error: $($error_bound)*,
            L: $logger_trait,
        {
            log!(logger, "[scan] Scanning I2C bus with control bytes: {:?}", control_bytes);
            match internal_scan(i2c, logger, control_bytes) {
                Ok(found_addrs) => {
                    for addr in found_addrs {
                        log!(logger, "[ok] Found device at 0x{:02X} (ctrl bytes: {:?})", addr, control_bytes);
                    }
                }
                Err(e) => log!(logger, "[error] I2C scan failed: {:?}", e),
            }
            log!(logger, "[info] I2C scan complete.");
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
        /// * `logger` - Mutable reference to a logger implementing the [`Logger`] trait.
        /// * `init_sequence` - Slice of initialization commands to test.
        ///
        /// # Example
        ///
        /// ```ignore
        /// use embedded_hal::i2c::I2c;
        /// use dvcdbg::logger::SerialLogger;
        /// use dvcdbg::scanner::scan_init_sequence;
        ///
        /// let mut i2c = /* your i2c interface */;
        /// let mut logger = SerialLogger::new(/* serial */);
        ///
        /// let init_sequence: [u8; 3] = [0xAE, 0xA1, 0xAF]; // example init cmds
        /// scan_init_sequence(&mut i2c, &mut logger, &init_sequence);
        /// ```
        pub fn scan_init_sequence<I2C, L>(
            i2c: &mut I2C,
            logger: &mut L,
            init_sequence: &[u8],
        ) where
            I2C: $i2c_trait,
            <I2C as $i2c_trait>::Error: $($error_bound)*,
            L: $logger_trait,
        {
            log!(logger, "[scan] Scanning I2C bus with init sequence: {:02X?}", init_sequence);
            let mut detected_cmds: heapless::Vec<u8, 64> = heapless::Vec::new();

            for &cmd in init_sequence {
                log!(logger, "-> Testing command 0x{:02X}", cmd);
                match internal_scan(i2c, logger, &[0x00, cmd]) {
                    Ok(found_addrs) => {
                        if !found_addrs.is_empty() {
                            for addr in found_addrs {
                                log!(logger, "[ok] Found device at 0x{:02X} responding to command 0x{:02X}", addr, cmd);
                            }
                            if detected_cmds.push(cmd).is_err() {
                                log!(logger, "[warn] Detected commands buffer is full, results may be incomplete!");
                            }
                        }
                    }
                    Err(e) => {
                        let _msg = $crate::recursive_log!("scan failed for command 0x{:02X}: {:?}", cmd, e);
                        log!(logger, "[error] {}", _msg);
                    }
                }
            }

            super::log_differences(logger, init_sequence, &detected_cmds);
            log!(logger, "[info] I2C scan with init sequence complete.");
        }

        fn internal_scan<I2C, L>(
            i2c: &mut I2C,
            logger: &mut L,
            data: &[u8],
        ) -> Result<heapless::Vec<u8, 128>, <I2C as $i2c_trait>::Error>
        where
            I2C: $i2c_trait,
            <I2C as $i2c_trait>::Error: $($error_bound)*,
            L: $logger_trait,
        {
            let mut found_devices: heapless::Vec<u8, 128> = heapless::Vec::new();

            for addr in super::I2C_SCAN_ADDR_START..=super::I2C_SCAN_ADDR_END {
                match i2c.write(addr, data) {
                    Ok(_) => {
                        found_devices.push(addr).unwrap(); // END - START < 128
                    }
                    Err(e) => {
                        if self::is_expected_nack(&e) {
                            // Not connect devices
                            continue;
                        } else {
                            let _msg = $crate::recursive_log!("write failed at 0x{:02X}: {:?}", addr, e);
                            log!(logger, "[error] {}", _msg);
                            return Err(e);
                        }
                    }
                }
            }

            Ok(found_devices)
        }
        
        #[cfg(feature = "ehal_1_0")]
        fn is_expected_nack<E>(err: &E) -> bool
        where
            E: $($error_bound)*,
        {
            use embedded_hal_1::i2c::ErrorKind;
            matches!(err.kind(), ErrorKind::NoAcknowledge(_))
        }

        #[cfg(feature = "ehal_0_2")]
        fn is_expected_nack<E>(err: &E) -> bool
        where
            E: $($error_bound)*,
        {
            let s = $crate::recursive_log!("{:?}", err);
            s.contains("NACK") || s.contains("NoAcknowledge")
        }
    };
}

// -----------------------------------------------------------------------------
//  Common utilities
// -----------------------------------------------------------------------------
use core::fmt::Debug;

pub trait I2cCompat {
    type Error: Debug;
    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error>;
}

fn log_differences<L>(logger: &mut L, expected: &[u8], detected: &Vec<u8, 64>)
where
    L: Logger,
{
    log!(logger, "Expected sequence: {:02X?}", expected);
    log!(
        logger,
        "Commands with response: {:02X?}",
        detected.as_slice()
    );

    let mut sorted = detected.clone();
    sorted.sort_unstable();
    let mut missing_cmds: Vec<u8, 64> = Vec::new();
    for cmd in expected
        .iter()
        .copied()
        .filter(|c| sorted.binary_search(c).is_err())
    {
        if missing_cmds.push(cmd).is_err() {
            log!(
                logger,
                "[warn] Missing commands buffer is full, list is truncated."
            );
            break;
        }
    }
    log!(
        logger,
        "Commands with no response: {:02X?}",
        missing_cmds.as_slice()
    );
}
