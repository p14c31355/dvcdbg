//! # dvcdbg Macros
//!
//! This module contains a collection of useful macros for embedded environments.
//! - Convert UART/Serial type to `core::fmt::Write`
//! - Hexadecimal representation of byte sequence
//! - I2C scan
//! - Debugging assistance (assert, delayed loop, cycle measurement)
//!

/// Implements `core::fmt::Write` for any serial type.
///
/// # Arguments
/// - `$type`: Target type (e.g., `arduino_hal::DefaultSerial`)
/// - `$write_method`: 1-byte send method (e.g., `write_byte`, `write`)
///
/// # Example
/// ```ignore
/// impl_fmt_write_for_serial!(arduino_hal::DefaultSerial, write_byte);
/// impl_fmt_write_for_serial!(esp_idf_hal::uart::UartDriver, write);
/// ```
#[macro_export]
macro_rules! impl_fmt_write_for_serial {
    ($type:ty, $write_method:ident) => {
        impl core::fmt::Write for $type {
            #[inline(always)]
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for &b in s.as_bytes() {
                    if self.$write_method(b).is_err() {
                        return Err(core::fmt::Error);
                    }
                }
                Ok(())
            }
        }
    };
}

/// Writes a byte slice in hexadecimal format to a `fmt::Write` target.
///
/// # Example
/// ```ignore
/// let buf = [0x12, 0xAB, 0xFF];
/// write_hex!(logger, &buf);
/// ```
#[macro_export]
macro_rules! write_hex {
    ($dst:expr, $data:expr) => {
        for &b in $data {
            let _ = core::write!($dst, "{:02X} ", b);
        }
    };
}

/// Writes a byte slice in binary format to a `fmt::Write` target.
///
/// Each byte is printed as an 8-bit binary number followed by a space.
///
/// # Arguments
/// - `$dst`: Destination implementing `core::fmt::Write`
/// - `$data`: Slice of bytes to print
///
/// # Example
/// ```ignore
/// let buf = [0b10101010, 0b11110000];
/// write_bin!(logger, &buf);
/// // Output: "10101010 11110000 "
/// ```
#[macro_export]
macro_rules! write_bin {
    ($dst:expr, $data:expr) => {
        for &b in $data {
            let _ = core::write!($dst, "{:08b} ", b);
        }
    };
}


/// Measures execution cycles (or timestamps) for an expression using a timer.
///
/// # Example
/// ```ignore
/// let (result, elapsed) = measure_cycles!(my_func(), timer);
/// ```
#[macro_export]
macro_rules! measure_cycles {
    ($expr:expr, $timer:expr) => {{
        let start = $timer.now();
        let result = $expr;
        let elapsed = $timer.now().wrapping_sub(start);
        (result, elapsed)
    }};
}

/// Runs a loop with a fixed delay between iterations.
///
/// # Example
/// ```ignore
/// loop_with_delay!(delay, { blink_led(); });
/// ```
#[macro_export]
macro_rules! loop_with_delay {
    ($delay:expr, $body:block) => {
        loop {
            $body
            $delay.delay_ms(1000u32);
        }
    };
}

/// Logs a simple assertion failure to a logger without panicking.
///
/// # Example
/// ```ignore
/// assert_log!(x == 42, logger, "Unexpected value: {}", x);
/// ```
#[macro_export]
macro_rules! assert_log {
    ($cond:expr, $logger:expr, $($arg:tt)*) => {
        if !$cond {
            let _ = core::write!($logger, "ASSERT FAILED: ");
            let _ = core::writeln!($logger, $($arg)*);
        }
    };
}

