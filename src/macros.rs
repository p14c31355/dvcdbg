//! # dvcdbg Macros
//!
//! This module contains a collection of useful macros for embedded environments.
//! - Convert UART/Serial type to `core::fmt::Write`
//! - Hexadecimal representation of byte sequence
//! - I2C scan
//! - Debugging assistance (assert, delayed loop, cycle measurement)
//!

/// ---------------------------------------------------------------------------
/// adapt_serial!
///
/// Purpose of the macro:
/// - Make serial ports such as UART/USART compatible with `embedded-io::Write` and `core::fmt::Write`.
/// - Also supports 1-byte write API using `nb` (avr-hal, etc.)
///
/// Usage (2 patterns):
///
/// 1) Types with 1-byte write in nb mode (e.g., `.write(u8) -> nb::Result<(), E>`)
///    adapt_serial!(UsartAdapter, nb_write = write);
///    // If flush is available
///    adapt_serial!(UsartAdapter, nb_write = write, flush = flush);
///
/// 2) If you only want to wrap a type that already implements `embedded_io::Write`
///    adapt_serial!(UsartAdapter, io_passthrough);
///
/// Generated code:
/// - `pub struct <Adapter<T>>(pub T)`
/// - `impl core::fmt::Write for <Adapter<T>>`
/// - `impl embedded_io::Write  for <Adapter<T>>`
///
/// Caution:
/// - In the nb backend, we have simplified the implementation by setting `type Error = core::convert::Infallible`.
/// ---------------------------------------------------------------------------
#[macro_export]
macro_rules! adapt_serial {
    // -------- Backend with 1-byte write that returns nb::Result<()> --------
    ($name:ident, nb_write = $write_fn:ident $(, flush = $flush_fn:ident)? ) => {
        pub struct $name<T>(pub T);

        impl<T> core::fmt::Write for $name<T> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                // Pack 1 byte at a time with nb::block macro
                for &b in s.as_bytes() {
                    let _ = nb::block!(self.0.$write_fn(b)).map_err(|_| core::fmt::Error)?;
                }
                Ok(())
            }
        }

        impl<T> embedded_io::Write for $name<T> {
            // Since most nb implementations do not actually fail, let's first decide on Infallible.
            type Error = core::convert::Infallible;

            fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
                for &b in buf {
                    // Discard the error and proceed
                    let _ = nb::block!(self.0.$write_fn(b));
                }
                Ok(buf.len())
            }

            fn flush(&mut self) -> Result<(), Self::Error> {
                $(
                    // Call only if there is a flush method.
                    let _ = nb::block!(self.0.$flush_fn());
                )?
                Ok(())
            }
        }
    };

    // -------- Simply wrap what has already been implemented in embedded-io::Write --------
    ($name:ident, io_passthrough) => {
        pub struct $name<T>(pub T);

        impl<T: embedded_io::Write> embedded_io::Write for $name<T> {
            type Error = <T as embedded_io::ErrorType>::Error;

            fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
                self.0.write(buf)
            }
            fn flush(&mut self) -> Result<(), Self::Error> {
                self.0.flush()
            }
        }

        impl<T: embedded_io::Write> core::fmt::Write for $name<T> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                // write_all compatible
                let mut left = s.as_bytes();
                while !left.is_empty() {
                    match self.0.write(left) {
                        Ok(0) => return Err(core::fmt::Error),
                        Ok(n) => left = &left[n..],
                        Err(_) => return Err(core::fmt::Error),
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
/// loop_with_delay!(delay, 100, { blink_led(); });
/// ```
#[macro_export]
macro_rules! loop_with_delay {
    ($delay:expr, $delay_ms:expr, $body:block) => {
        loop {
            $body
            $delay.delay_ms($delay_ms);
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

/// Scans I2C bus for devices and logs found addresses.
///
/// # Example
/// ```ignore
/// scan_i2c!(i2c, logger);
/// ```
#[macro_export]
macro_rules! scan_i2c {
    ($i2c:expr, $logger:expr) => {{
        $crate::scanner::scan_i2c($i2c, $logger);
    }};
}

/// Quick diagnostic workflow for a new board.
///
/// Automatically performs:
/// 1. Serial logger check
/// 2. I2C bus scan
/// 3. Optional cycle measurement of a test expression
///
/// # Arguments
/// - `$serial`: Serial logger implementing `core::fmt::Write`
/// - `$i2c`: I2C bus instance
/// - `$timer`: Timer implementing `.now()`
/// - `$test_expr`: Optional expression to measure cycles for (can be `{}` block)
///
/// # Example
/// ```ignore
/// quick_diag!(logger, i2c, timer, { my_func(); });
/// ```
#[macro_export]
macro_rules! quick_diag {
    ($serial:expr, $i2c:expr, $timer:expr, $test_expr:block) => {{
        quick_diag!(@inner $serial, $i2c);

        // Test expression timing
        let (_result, cycles) = $crate::measure_cycles!($test_expr, $timer);
        let _ = core::writeln!($serial, "Test expression cycles: {}", cycles);

        let _ = core::writeln!($serial, "=== Quick Diagnostic Complete ===");
    }};
    ($serial:expr, $i2c:expr) => {{
        quick_diag!(@inner $serial, $i2c);
        let _ = core::writeln!($serial, "=== Quick Diagnostic Complete ===");
    }};
    // Internal rule for common diagnostic steps.
    (@inner $serial:expr, $i2c:expr) => {
        let _ = core::writeln!($serial, "=== Quick Diagnostic Start ===");
        // I2C bus scan
        let _ = core::writeln!($serial, "Scanning I2C bus...");
        $crate::scan_i2c!($i2c, $serial);
    };
}
