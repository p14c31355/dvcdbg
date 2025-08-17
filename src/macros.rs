//! # dvcdbg Macros
//!
//! This module contains a collection of useful macros for embedded environments.
//! - Convert UART/Serial type to `core::fmt::Write`
//! - Hexadecimal representation of byte sequence
//! - I2C scan
//! - Debugging assistance (assert, delayed loop, cycle measurement)
//!

/// Macro to adapt a serial peripheral into a fmt::Write + embedded_io::Write bridge.
///
/// # Variants
/// - nb_write: wraps `nb`-style `write(byte)`
/// - io_passthrough: wraps `embedded_io::Write` directly
///
/// # Example:
/// ```ignore
/// adapt_serial!(UsartAdapter, nb_write = write, error = nb::Error<Infallible>, flush = flush);
/// let mut uart = UsartAdapter(serial);
/// writeln!(uart, "Hello!").unwrap();
/// uart.write(&[0x01, 0x02]).unwrap();
/// ```
#[macro_export]
macro_rules! adapt_serial {
    // nb_write variant with optional flush
    ($name:ident, nb_write = $write_fn:ident, error = $err_ty:ty $(, flush = $flush_fn:ident)?) => {
        pub struct $name<T>(pub T);

        impl<T> embedded_io::Write for $name<T> {
            type Error = $err_ty;

            fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
                for &b in buf {
                    nb::block!(self.0.$write_fn(b))?;
                }
                Ok(buf.len())
            }

            fn flush(&mut self) -> Result<(), Self::Error> {
                $(
                    nb::block!(self.0.$flush_fn())?;
                )?
                Ok(())
            }
        }

        impl<T> core::fmt::Write for $name<T>
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                use embedded_io::Write;
                self.write_all(s.as_bytes())
                    .map_err(|_| core::fmt::Error)
            }
            fn flush(&mut self) -> core::fmt::Result {
                use embedded_io::Write;
                self.flush().map_err(|_| core::fmt::Error)
            }
        }
    };

    // passthrough variant for types that already implement embedded_io::Write
    ($name:ident, io_passthrough) => {
        pub struct $name<T>(pub T);

        impl<T: embedded_io::Write> embedded_io::Write for $name<T> {
            type Error = T::Error;

            fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
                self.0.write(buf)
            }

            fn flush(&mut self) -> Result<(), Self::Error> {
                self.0.flush()
            }
        }

        impl<T: embedded_io::Write> core::fmt::Write for $name<T> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                self.0.write_all(s.as_bytes())
                    .map_err(|_| core::fmt::Error)
            }

            fn flush(&mut self) -> core::fmt::Result {
                self.0.flush().map_err(|_| core::fmt::Error)
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
