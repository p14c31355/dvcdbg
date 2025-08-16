//! # dvcdbg Macros
//!
//! This module contains a collection of useful macros for embedded environments.
//! - Convert UART/Serial type to `core::fmt::Write`
//! - Hexadecimal representation of byte sequence
//! - I2C scan
//! - Debugging assistance (assert, delayed loop, cycle measurement)
//!

/// Internal macro: Implements `embedded-hal::serial::Write<u8>` for the given type.
macro_rules! __impl_write_trait {
    ($ty:ty, $write_fn:ident) => {
        impl embedded_hal::serial::Write<u8> for $ty {
            type Error = core::convert::Infallible;

            #[inline(always)]
            fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
                self.$write_fn(word);
                Ok(())
            }

            #[inline(always)]
            fn flush(&mut self) -> nb::Result<(), Self::Error> {
                Ok(())
            }
        }

        impl core::fmt::Write for $ty {
            #[inline(always)]
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for &b in s.as_bytes() {
                    // If transmission fails, immediately return Err.
                    if self.$write_fn(b).is_err() {
                        return Err(core::fmt::Error);
                    }
                }
                Ok(())
            }
        }
    };
}

/// Wraps a HAL-specific serial type and implements both
/// [`embedded_hal::serial::Write<u8>`] and [`core::fmt::Write`].
///
/// This allows using any serial peripheral as a backend for
/// `dvcdbg` logging or standard [`write!`] / [`writeln!`] macros.
///
/// # Variants
///
/// - `avr_usart`: Special case for [`arduino-hal`] USARTs, which use
///   4 generic parameters (`Usart<U, RX, TX, CLOCK>`).
/// - `generic`: Any other serial type (STM32, RP2040, ESP-IDF, etc.)
///   where the type is monomorphic or already generic-safe.
///
/// # Arguments
///
/// - `$wrapper`: The new wrapper type name you want to expose.
/// - `$target`: Target type (for `generic` only).
/// - `$write_fn`: Method on the target type that writes one byte
///   (e.g., `write`, `write_byte`).
///
/// # Examples
///
/// ## Arduino Uno (avr-hal)
/// ```no_run
/// use dvcdbg::adapt_serial;
///
/// adapt_serial!(avr_usart: UsartAdapter, write_byte);
///
/// fn main() {
///     let dp = arduino_hal::Peripherals::take().unwrap();
///     let mut serial = arduino_hal::default_serial!(dp, 57600);
///
///     let mut dbg_uart = UsartAdapter(serial);
///
///     // usable as dvcdbg backend
///     dvcdbg::adapter!(dbg_uart);
///
///     // also usable with core::fmt::write!
///     use core::fmt::Write;
///     writeln!(dbg_uart, "Hello from AVR!").ok();
/// }
/// ```
///
/// ## STM32 (stm32f4xx-hal)
/// ```ignore
/// use dvcdbg::adapt_serial;
///
/// adapt_serial!(generic: UartAdapter, stm32f4xx_hal::serial::Tx<USART1>, write);
///
/// let tx: stm32f4xx_hal::serial::Tx<USART1> = /* init */;
/// let mut dbg_uart = UartAdapter(tx);
///
/// dvcdbg::adapter!(dbg_uart);
/// writeln!(dbg_uart, "stm32 log").ok();
/// ```
///
/// ## ESP-IDF (esp-idf-hal)
/// ```ignore
/// use dvcdbg::adapt_serial;
///
/// adapt_serial!(generic: EspAdapter, esp_idf_hal::uart::UartDriver, write);
///
/// let uart = esp_idf_hal::uart::UartDriver::new(/* ... */)?;
/// let mut dbg_uart = EspAdapter(uart);
///
/// dvcdbg::adapter!(dbg_uart);
/// writeln!(dbg_uart, "esp32 log").ok();
/// ```
#[macro_export]
macro_rules! adapt_serial {
    // avr-hal's Usart (with generics)
    (avr_usart: $wrapper:ident, $write_fn:ident) => {
        pub struct $wrapper<U, RX, TX, CLOCK>(
            pub arduino_hal::hal::usart::Usart<U, RX, TX, CLOCK>
        );
        $crate::__impl_write_trait!($wrapper<U, RX, TX, CLOCK>, $write_fn);
    };

    // Generic (without generics)
    (generic: $wrapper:ident, $target:ty, $write_fn:ident) => {
        pub struct $wrapper(pub $target);
        $crate::__impl_write_trait!($wrapper, $write_fn);
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
