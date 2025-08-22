//! Scanner utilities for I2C bus device discovery and analysis.
//!
//! This module provides functions to scan the I2C bus for connected devices,
//! optionally testing with control bytes or initialization command sequences,
//! with detailed logging support.

use crate::log;
use crate::logger::Logger;
use heapless::Vec;

pub const I2C_SCAN_ADDR_START: u8 = 0x03;
pub const I2C_SCAN_ADDR_END: u8 = 0x77;

#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
pub mod ehal_0_2 {
    use crate::define_scanner;

    define_scanner!(
        crate::compat::I2cCompat,
        crate::logger::Logger
    );
}

#[cfg(feature = "ehal_1_0")]
pub mod ehal_1_0 {
    use crate::define_scanner;
    
    define_scanner!(
        crate::compat::I2cCompat,
        crate::logger::Logger
    );
}

#[cfg(feature = "ehal_1_0")]
pub use self::ehal_1_0::{scan_i2c, scan_i2c_with_ctrl, scan_init_sequence};

#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
pub use self::ehal_0_2::{scan_i2c, scan_i2c_with_ctrl, scan_init_sequence};

#[macro_export]
macro_rules! define_scanner {
    ($i2c_trait:path, $logger_trait:path) => {
        use $crate::error::{ErrorKind, I2cError};
        use $crate::compat::HalErrorExt;
        use $crate::log;
        use heapless::Vec;
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
            L: $logger_trait,
            <I2C as $i2c_trait>::Error: HalErrorExt,
        {
            log!(logger, "[scan] Scanning I2C bus...");
            if let Ok(found_addrs) = internal_scan(i2c, logger, &[]) {
                if !found_addrs.is_empty() {
                    use core::fmt::Write;
                    let mut addrs_str = heapless::String::<640>::new();
                    for addr in found_addrs {
                        if write!(addrs_str, "0x{:02X} ", addr).is_err() {
                            let _ = addrs_str.push_str("...");
                            break;
                        }
                    }
                    log!(logger, "[ok] Found devices at: {}", addrs_str.trim_end());
                }
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
            L: $logger_trait,
            <I2C as $i2c_trait>::Error: HalErrorExt,
        {
            use core::fmt::Write;
            let mut s = heapless::String::<256>::new();
            for &b in control_bytes {
                if write!(s, "0x{:02X} ", b).is_err() {
                    let _ = s.push_str("...");
                    break;
                }
            }
            log!(logger, "[scan] Scanning I2C bus with control bytes: {}", s.trim_end());

            if let Ok(found_addrs) = internal_scan(i2c, logger, control_bytes) {
                if !found_addrs.is_empty() {
                    let addrs_str: heapless::String<384> = super::bytes_to_hex_str(&found_addrs);
                    log!(logger, "[ok] Found devices at: {}", addrs_str);
                }
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
            L: $logger_trait,
            <I2C as $i2c_trait>::Error: HalErrorExt,
        {
            let s: heapless::String<256> = super::bytes_to_hex_str(init_sequence);
            log!(logger, "[scan] Scanning I2C bus with init sequence: {}", s);

            let mut detected_cmds: Vec<u8, 64> = Vec::new();
            for &cmd in init_sequence {
                match internal_scan(i2c, logger, &[0x00, cmd]) {
                    Ok(found_addrs) => {
                        if !found_addrs.is_empty() {
                            for addr in found_addrs {
                                log!(logger, "[ok] Found device at 0x{:02X} responding to 0x{:02X}", addr, cmd);
                            }
                            if detected_cmds.push(cmd).is_err() {
                                log!(logger, "[warn] Detected commands buffer is full, results may be incomplete!");
                            }
                        }
                    }
                    Err(e) => {
                        log!(logger, "[error] scan failed for 0x{:02X}: {:?}", cmd, e);
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
        ) -> Result<Vec<u8, 128>, ErrorKind>
        where
            I2C: $i2c_trait,
            L: $logger_trait,
            <I2C as $i2c_trait>::Error: HalErrorExt,
        {
            let mut found_devices: Vec<u8, 128> = Vec::new();

            for addr in super::I2C_SCAN_ADDR_START..=super::I2C_SCAN_ADDR_END {
                match i2c.write(addr, data) {
                    Ok(_) => {
                        let _ = found_devices.push(addr);
                    }
                    Err(e) => {
                        let e_kind = e.to_compat(Some(addr));
                        if e_kind == ErrorKind::I2c(I2cError::Nack) {
                            continue;
                        } else {
                            use core::fmt::Write;

                            let mut err_str = heapless::String::<64>::new();
                            let _ = match e_kind {
                                ErrorKind::I2c(I2cError::ArbitrationLost) => err_str.push_str("ArbitrationLost"),
                                ErrorKind::I2c(I2cError::Bus) => err_str.push_str("BusError"),
                                ErrorKind::Other => err_str.push_str("Other"),
                                _ => write!(err_str, "{:?}", e_kind),
                            };
                            log!(logger, "[error] write failed at 0x{:02X}: {}", addr, err_str);
                            return Err(e_kind);
                        }
                    }
                }
            }

            Ok(found_devices)
        }

    }
}

fn log_differences<L>(logger: &mut L, expected: &[u8], detected: &Vec<u8, 64>)
where
    L: Logger,
{
    use core::fmt::Write;
    let mut s = heapless::String::<384>::new();
    for &b in expected {
        if write!(s, "0x{b:02X} ").is_err() {
            let _ = s.push_str("...");
            break;
        }
    }
    log!(logger, "Expected sequence: {}", s.trim_end());
    s.clear();
    for &b in detected.as_slice() {
        if write!(s, "0x{b:02X} ").is_err() {
            let _ = s.push_str("...");
            break;
        }
    }
    log!(logger, "Commands with response: {}", s.trim_end());

    let mut sorted = detected.clone();
    sorted.sort_unstable();
    let mut missing_cmds: Vec<u8, 64> = Vec::new();
    for cmd in expected.iter().copied().filter(|c| sorted.binary_search(c).is_err()) {
        if missing_cmds.push(cmd).is_err() {
            log!(
                logger,
                "[warn] Missing commands buffer is full, list is truncated."
            );
            break;
        }
    }

    s.clear();
    for &b in missing_cmds.as_slice() {
        if write!(s, "0x{b:02X} ").is_err() {
            let _ = s.push_str("...");
            break;
        }
    }
    log!(logger, "Commands with no response: {}", s.trim_end());
}

fn bytes_to_hex_str<const N: usize>(bytes: &[u8]) -> heapless::String<N> {
    use core::fmt::Write;
    let mut s = heapless::String::<N>::new();
    for &b in bytes {
        if write!(&mut s, "0x{b:02X} ").is_err() {
            let _ = s.push_str("...");
            return s;
        }
    }
    if !s.is_empty() {
        s.pop(); // Remove trailing space
    }
    s
}