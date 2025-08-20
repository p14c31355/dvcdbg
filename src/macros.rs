//! # dvcdbg Macros
//!
//! This module contains a collection of useful macros for embedded environments.
//! - Convert UART/Serial type to `core::fmt::Write`
//! - Hexadecimal representation of byte sequence
//! - I2C scan
//! - Debugging assistance (assert, delayed loop, cycle measurement)
//!

/// Wrap a type implementing `SerialCompat` and provide a `core::fmt::Write` adapter.
///
/// # Purpose
///
/// The `adapt_serial!` macro generates a newtype wrapper around a type `T` that implements
/// [`SerialCompat`]. This wrapper allows you to:
/// 
/// 1. Access a `CoreWriteAdapter` via `as_core_write()` for integration with `core::fmt::Write`.
/// 2. Directly use the wrapper as a `core::fmt::Write` object for formatted output.
///
/// This is useful for logging or printing to serial peripherals in a `no_std` context
/// without depending directly on HAL-specific traits.
///
/// # Example
///
/// ```ignore
/// use dvcdbg::compat::serial_compat::SerialCompat;
/// use dvcdbg::compat::adapt::CoreWriteAdapter;
/// 
/// // Suppose `MySerial` implements `SerialCompat`
/// struct MySerial;
/// impl SerialCompat for MySerial {
///     type Error = ();
///     fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> { Ok(()) }
///     fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
/// }
///
/// // Generate a wrapper type
/// adapt_serial!(MySerialAdapter);
///
/// let mut serial = MySerial;
/// let mut wrapper = MySerialAdapter(serial);
///
/// // Use core::fmt macros
/// use core::fmt::Write;
/// writeln!(wrapper, "Hello, world!").unwrap();
///
/// // Or get a CoreWriteAdapter directly
/// let mut adapter: CoreWriteAdapter<_> = wrapper.as_core_write();
/// writeln!(adapter, "Direct CoreWriteAdapter usage").unwrap();
/// ```
///
/// # Notes
///
/// - The generated wrapper struct is generic over `T` and requires `T: SerialCompat`.
/// - This macro is `#[macro_export]` so it can be used across crates.
/// - Provides zero-cost abstraction over `SerialCompat` for `core::fmt::Write` output.
#[macro_export]
macro_rules! adapt_serial {
    ($name:ident) => {
        pub struct $name<T>(pub T);

        impl<T> $name<T>
        where
            T: $crate::compat::serial_compat::SerialCompat,
        {
            /// Return a `CoreWriteAdapter` that implements `core::fmt::Write`.
            pub fn as_core_write(&mut self) -> $crate::compat::adapt::CoreWriteAdapter<&mut Self> {
                $crate::compat::adapt::CoreWriteAdapter(self)
            }
        }

        impl<T> embedded_io::ErrorType for $name<T>
        where
            T: $crate::compat::serial_compat::SerialCompat,
        {
            type Error = $crate::compat::serial_compat::CompatErr<T::Error>;
        }

        impl<T> embedded_io::Write for $name<T>
        where
            T: $crate::compat::serial_compat::SerialCompat,
        {
            fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
                self.0.write(buf).map_err($crate::compat::serial_compat::CompatErr)?;
                Ok(buf.len())
            }

            fn flush(&mut self) -> Result<(), Self::Error> {
                self.0.flush().map_err($crate::compat::serial_compat::CompatErr)
            }
        }

        impl<T> core::fmt::Write for $name<T>
        where
            T: $crate::compat::serial_compat::SerialCompat,
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                // Now that the adapter implements `embedded_io::Write`, we can use `write_all`.
                use embedded_io::Write;
                self.write_all(s.as_bytes()).map_err(|_| core::fmt::Error)
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
