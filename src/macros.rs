//! # dvcdbg Macros
//!
//! This module contains a collection of useful macros for embedded environments.
//! - Convert UART/Serial type to `core::fmt::Write`
//! - Hexadecimal representation of byte sequence
//! - I2C scan
//! - Debugging assistance (assert, delayed loop, cycle measurement)
//!

/// Wraps a serial peripheral that does **not** implement `embedded-hal::serial::Write<u8>`
/// and provides implementations for:
/// - [`core::fmt::Write`] → allows using `write!` / `writeln!`
/// - [`embedded_hal::blocking::serial::Write`] → safe for blocking serials
///
/// # Variants
///
/// - `avr_usart`: For `arduino-hal` USARTs on AVR MCUs (`U, RX, TX, CLOCK` generics).
/// - `generic`: For any type that provides a simple blocking `write_byte(&mut self, u8)`
///   method (e.g., STM32, RP2040, ESP32, or other HALs).
///
/// # Arguments
///
/// - `$wrapper`: Wrapper type name
/// - `$target`: Target type (only for `generic`)
/// - `$write_fn`: Method on the target that writes a single byte
///
/// # Examples
///
/// ## Arduino Uno (avr-hal)
/// ```ignore
/// use dvcdbg::adapt_serial;
///
/// adapt_serial!(avr_usart: UsartAdapter, write_byte);
///
/// let dp = arduino_hal::Peripherals::take().unwrap();
/// let mut serial = arduino_hal::default_serial!(dp, 57600);
/// let mut dbg_uart = UsartAdapter(serial);
///
/// // Use with dvcdbg
/// dvcdbg::adapter!(dbg_uart);
///
/// use core::fmt::Write;
/// writeln!(dbg_uart, "Hello AVR!").ok();
/// ```
///
/// ## Generic blocking serial (STM32 / RP2040 / ESP32)
/// ```ignore
/// struct MySerial;
/// impl MySerial {
///     fn write_byte(&mut self, b: u8) -> Result<(), ()> { Ok(()) }
/// }
///
/// adapt_serial!(generic: MyAdapter, MySerial, write_byte);
/// let mut uart = MyAdapter(MySerial);
/// writeln!(uart, "Logging via generic serial").ok();
/// ```
#[macro_export]
macro_rules! adapt_serial {
    // Internal helper to generate the impl blocks
    (@impls $wrapper:ident, $write_fn:ident, <$($generics:tt)*> $(, $where:tt)?) => {
        impl<$($generics)*> embedded_hal::blocking::serial::Write<u8>
            for $wrapper<$($generics)*>
            $( $where )?
        {
            type Error = ();

            fn bwrite_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
                for &b in buffer {
                    self.0.$write_fn(b).map_err(|_| ())?;
                }
                Ok(())
            }

            fn bflush(&mut self) -> Result<(), Self::Error> {
                Ok(())
            }
        }

        impl<$($generics)*> core::fmt::Write for $wrapper<$($generics)*>
            $( $where )?
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                use embedded_hal::blocking::serial::Write;
                self.bwrite_all(s.as_bytes()).map_err(|_| core::fmt::Error)
            }
        }
    };

    // AVR-HAL USART (4 generics)
    (avr_usart: $wrapper:ident, $write_fn:ident) => {
        pub struct $wrapper<
            U,
            RX,
            TX,
            CLOCK
        >(
            pub arduino_hal::hal::usart::Usart<U, RX, TX, CLOCK>
        );

        adapt_serial!(
            @impls $wrapper, $write_fn,
            <U, RX, TX, CLOCK>,
            where U: arduino_hal::hal::usart::UsartOps<
                arduino_hal::atmega_hal::Atmega, RX, TX
            >
        );
    };

    // Generic serial type with blocking write method
    (generic: $wrapper:ident, $target:ty, $write_fn:ident) => {
        pub struct $wrapper(pub $target);

        adapt_serial!(@impls $wrapper, $write_fn, <> ,);
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
