//! # dvcdbg Macros
//!
//! This module contains a collection of useful macros for embedded environments.
//! - Convert UART/Serial type to `core::fmt::Write`
//! - Hexadecimal representation of byte sequence
//! - I2C scan
//! - Debugging assistance (assert, delayed loop, cycle measurement)
//!

/// # adapt_serial! macro
///
/// Wraps any serial peripheral implementing a `write_byte(&mut self, u8)`
/// method into a type that implements:
/// - [`core::fmt::Write`] → allows `write!` / `writeln!`
/// - [`embedded_hal::blocking::serial::Write<u8>`] → safe blocking write
///
/// # Variants
///
/// - `avr_usart`: For `arduino-hal` USARTs (ATmega) with 3 generics (`RX, TX, CLOCK`)
/// - `generic`: For any type that has a simple blocking `write_byte` method
///
/// # Arguments
///
/// - `$wrapper`: Wrapper type name
/// - `$write_fn`: Method on the target that writes a single byte
///
/// # Examples
///
/// ```ignore
/// // AVR USART
/// let mut usart0: arduino_hal::Usart0 = ...;
/// adapt_serial!(avr: UsartAdapter: usart0, write_byte);
/// let mut uart = UsartAdapter(usart0);
/// writeln!(uart, "Hello AVR!").ok();
///
/// // ESP-IDF UART
/// let esp_uart: EspUart = ...;
/// adapt_serial!(generic: EspAdapter: esp_uart, write_byte);
/// let mut logger = EspAdapter(esp_uart);
/// writeln!(logger, "Hello ESP!").ok();
/// ```
///
/// ## Generic blocking serial
/// ```ignore
/// struct MySerial;
/// impl MySerial {
///     fn write_byte(&mut self, b: u8) -> Result<(), ()> { Ok(()) }
/// }
///
/// adapt_serial!(generic: my_adapter: MyAdapter, MySerial, write_byte);
///
/// let mut uart = MyAdapter(MySerial);
/// writeln!(uart, "Logging via generic serial").ok();
/// ```
#[macro_export]
macro_rules! adapt_serial {
    // common implementation: core::fmt::Write + embedded_hal::blocking::serial::Write<u8>
    (@impls $wrapper:ty, $write_fn:ident) => {
        impl core::fmt::Write for $wrapper {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for &b in s.as_bytes() {
                    self.$write_fn(b).map_err(|_| core::fmt::Error)?;
                }
                Ok(())
            }
        }

        impl embedded_hal::blocking::serial::Write<u8> for $wrapper {
            type Error = ();

            fn bwrite_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
                for &b in buf {
                    self.$write_fn(b).map_err(|_| ())?;
                }
                Ok(())
            }

            fn bflush(&mut self) -> Result<(), Self::Error> { Ok(()) }
        }
    };

    // AVR: Wrapping USART with type generic
    (avr: $wrapper:ident: $instance:expr, $write_fn:ident) => {
        pub struct $wrapper<T>(pub T);
        impl<T> $wrapper<T> {
            pub fn new(inner: T) -> Self { Self(inner) }
        }

        impl<T> core::ops::Deref for $wrapper<T> {
            type Target = T;
            fn deref(&self) -> &Self::Target { &self.0 }
        }
        impl<T> core::ops::DerefMut for $wrapper<T> {
            fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
        }

        adapt_serial!(@impls $wrapper<$crate::core::marker::PhantomData>, $write_fn);
    };

    // Generic: Types with arbitrary write_byte methods
    (generic: $wrapper:ident: $target:ty, $write_fn:ident) => {
        pub struct $wrapper(pub $target);
        impl $wrapper {
            pub fn new(inner: $target) -> Self { Self(inner) }
        }
        adapt_serial!(@impls $wrapper, $write_fn);
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
