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
/// Supports both embedded-hal 0.2.x (nb::Write<u8>) and 1.0.x (embedded_hal::serial::nb::Write<u8>).
/// Serial adapter wrapper macro compatible with e-hal 0.2.x and 1.0
#[macro_export]
macro_rules! adapt_serial {
    ($name:ident) => {
        pub struct $name<T>(pub T);

        impl<T> $name<T>
        where
            T: $crate::compat::serial_compat::SerialCompat,
        {
            /// Return a `CoreWriteAdapter` that implements `core::fmt::Write`.
            pub fn as_core_write(&mut self) -> $crate::compat::adapt::CoreWriteAdapter<&mut T> {
                $crate::compat::adapt::CoreWriteAdapter(&mut self.0)
            }
        }

        impl<T> core::fmt::Write for $name<T>
        where
            T: $crate::compat::serial_compat::SerialCompat,
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let mut adapter = $crate::compat::adapt::CoreWriteAdapter(&mut self.0);
                adapter.write_str(s)
            }
        }
    };
}

/// # adapt_i2c! macro
///
/// Wraps an I2C peripheral that implements `embedded-hal::blocking::i2c`
/// (0.2) or `embedded-hal::i2c` (1.0) and provides `I2cCompat`.
/// 
/// Use the adapt_i2c! macro to implement your own HAL I2C → I2cCompat conversion.
/// This will allow you to pass it to common processes such as scan_i2c in dvcdbg.
///
/// # Example
/// ```ignore
/// adapt_i2c!(avr_i2c: I2cAdapter);
/// ```
#[macro_export]
macro_rules! adapt_i2c {
    ($adapter:ident) => {
        pub struct $adapter<'a, T: 'a>(&'a mut T)
        where
            T: $crate::compat::i2c_compat::I2cCompat;

        impl<'a, T> $crate::compat::i2c_compat::I2cCompat for $adapter<'a, T>
        where
            T: $crate::compat::i2c_compat::I2cCompat,
        {
            type Error = T::Error;

            fn write_read(
                &mut self,
                addr: u8,
                bytes: &[u8],
                buffer: &mut [u8],
            ) -> Result<(), Self::Error> {
                self.0.write_read(addr, bytes, buffer)
            }

            fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
                self.0.write(addr, bytes)
            }

            fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
                self.0.read(addr, buffer)
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
