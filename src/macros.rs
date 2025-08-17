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
/// # Arguments
/// - `$name` → Name of the generated adapter struct
/// - `nb_write` → Serial write method name required by HAL
/// - `flush` (optional) → Method for flushing non-blocking serial
///
/// # Example
///
/// ## Arduino HAL Serial
/// ```ignore
/// use arduino_hal::prelude::*;
/// use dvcdbg::adapt_serial;
/// use core::fmt::Write;
/// use embedded_io::Write;
///
/// adapt_serial!(UsartAdapter, nb_write = write, flush = flush);
///
/// let dp = arduino_hal::Peripherals::take().unwrap();
/// let pins = arduino_hal::pins!(dp);
/// let serial = arduino_hal::default_serial!(dp, pins, 57600);
/// let mut dbg_uart = UsartAdapter(serial);
///
/// writeln!(dbg_uart, "Hello from embedded-io bridge!").unwrap();
/// dbg_uart.write_all(&[0x01, 0x02, 0x03]).unwrap();
/// ```
///
/// ## Custom serial-like type
/// ```ignore
/// use dvcdbg::adapt_serial;
/// use core::fmt::Write;
/// use core::convert::Infallible;
/// use nb;
/// use embedded_io::Write;
///
/// struct MySerial;
/// impl nb::serial::Write<u8> for MySerial {
///     type Error = Infallible; // Error type is not fixed to Infallible
///     fn write(&mut self, _byte: u8) -> nb::Result<(), Self::Error> { Ok(()) }
///     fn flush(&mut self) -> nb::Result<(), Self::Error> { Ok(()) }
/// }
///
/// adapt_serial!(MyAdapter, nb_write = write, flush = flush);
/// let mut uart = MyAdapter(MySerial);
/// writeln!(uart, "Hello via custom serial").unwrap();
/// uart.write_all(&[0xAA, 0xBB]).unwrap();
/// ```
#[macro_export]
macro_rules! adapt_serial {
    ($name:ident, nb_write = $write_fn:ident $(, flush = $flush_fn:ident)?) => {
        /// Serial adapter wrapper
        pub struct $name<T>(pub T);

        /// Implement embedded-io Write for the wrapper
        impl<T> embedded_io::Write for $name<T>
        where
            T: nb::Write<u8>,
            <T as nb::Write<u8>>::Error: core::fmt::Debug,
        {
            type Error = $crate::AdaptError<<T as nb::Write<u8>>::Error>;
            fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
                for &b in buf {
                    nb::block!(self.0.$write_fn(b)).map_err($crate::AdaptError::Other)?;
                }
                Ok(buf.len())
            }
            fn flush(&mut self) -> Result<(), Self::Error> {
                $(nb::block!(self.0.$flush_fn()).map_err($crate::AdaptError::Other)?;)?
                Ok(())
            }
        }

        /// Implement core::fmt::Write for writeln! / write!
        impl<T> core::fmt::Write for $name<T>
        where
            T: nb::Write<u8>,
            <T as nb::Write<u8>>::Error: core::fmt::Debug,
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                <Self as embedded_io::Write>::write_all(self, s.as_bytes())
                    .map_err(|_| core::fmt::Error)
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
