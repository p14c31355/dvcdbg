//! logger.rs
//! Lightweight, feature-gated logger implementation for embedded environments.
//!
//! This module provides:
//! - A `Logger` trait for uniform logging
//! - A `log!` macro with `debug_log` feature gating
//! - Multiple logger implementations:
//!   - `SerialLogger`: For serial output (`core::fmt::Write`)
//!   - `BufferedLogger`: Keeps logs in memory (`heapless::String`)
//!   - `NoopLogger`: Discards all log output
//!
//! When the `debug_log` feature is **disabled**, the `log!` macro expands to nothing,
//! and all logging calls are removed at compile time.
//!
//! # Example
//! ```no_run
//! use dvcdbg::{log, logger::{Logger, SerialLogger}};
//!
//! struct DummyWriter(String);
//! impl core::fmt::Write for DummyWriter {
//!     fn write_str(&mut self, s: &str) -> core::fmt::Result {
//!         self.0.push_str(s);
//!         Ok(())
//!     }
//! }
//!
//! let mut dw = DummyWriter(String::new());
//! let mut logger = SerialLogger::new(&mut dw);
//! log!(logger, "Hello {}!", "world");
//! ``` 

#[macro_export]
macro_rules! log {
    ($logger:expr, $($arg:tt)*) => {
        #[cfg(feature = "debug_log")]
        {
            $logger.log_fmt(core::format_args!($($arg)*));
        }
    };
}

/// Common logging interface.
///
/// Implementors provide a `log_fmt()` method for formatted output.
/// Additional helper methods like [`log_bytes()`], [`log_i2c()`], and [`log_cmd()`]
/// are enabled only when the `debug_log` feature is active.
pub trait Logger {
    /// Logs a pre-formatted message.
    fn log_fmt(&mut self, args: core::fmt::Arguments);

    /// Logs a byte slice in `0xXX` format with a label.
    ///
    /// Truncates output with `...` if it exceeds the internal buffer.
    #[cfg(feature = "debug_log")]
    fn log_bytes(&mut self, label: &str, bytes: &[u8]) {
        use core::fmt::Write;
        let mut out: heapless::String<128> = heapless::String::new();
        let _ = write!(&mut out, "{label}: ");
        for b in bytes {
            if write!(&mut out, "0x{b:02X} ").is_err() {
                                let cap = out.capacity();
                if out.len() > cap.saturating_sub(3) {
                    out.truncate(cap.saturating_sub(3));
                }
                let _ = out.push_str("...");
                break;
            }
        }
        self.log_fmt(format_args!("{out}"));
    }

    /// Logs the result of an I2C transaction with a ✅/❌ marker.
    #[cfg(feature = "debug_log")]
    fn log_i2c(&mut self, context: &str, result: Result<(), impl core::fmt::Debug>) {
        match result {
            Ok(_) => self.log_fmt(format_args!("✅ {context} OK")),
            Err(e) => self.log_fmt(format_args!("❌ {context} FAILED: {e:?}")),
        }
    }

    /// Logs a single command byte in `0xXX` format.
    #[cfg(feature = "debug_log")]
    fn log_cmd(&mut self, cmd: u8) {
        self.log_fmt(format_args!("0x{cmd:02X}"));
    }
}

/// Logger that writes directly to any `core::fmt::Write` target.
pub struct SerialLogger<'a, W: core::fmt::Write>(&'a mut W);

impl<'a, W: core::fmt::Write> SerialLogger<'a, W> {
    /// Creates a new `SerialLogger` that writes to the given target.
    pub fn new(writer: &'a mut W) -> Self {
        Self(writer)
    }
}

impl<'a, W: core::fmt::Write> Logger for SerialLogger<'a, W> {
    fn log_fmt(&mut self, args: core::fmt::Arguments) {
        let _ = writeln!(self.0, "{args}");
    }
}

/// Logger that stores messages in a heapless string buffer.
///
/// Useful for testing or when logs must be retrieved later.
#[cfg(feature = "debug_log")]
pub struct BufferedLogger<const N: usize> {
    buffer: heapless::String<N>,
}

#[cfg(feature = "debug_log")]
impl<const N: usize> BufferedLogger<N> {
    /// Creates a new empty `BufferedLogger`.
    pub fn new() -> Self {
        Self {
            buffer: heapless::String::new(),
        }
    }

    /// Returns a string slice of all stored logs.
    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    /// Clears the stored log buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(feature = "debug_log")]
impl<const N: usize> Logger for BufferedLogger<N> {
    fn log_fmt(&mut self, args: core::fmt::Arguments) {
        use core::fmt::Write;
        let _ = writeln!(self.buffer, "{args}");
    }
}

/// Logger that discards all messages.
pub struct NoopLogger;

impl NoopLogger {
    /// Creates a new `NoopLogger` instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Logger for NoopLogger {
    fn log_fmt(&mut self, _: core::fmt::Arguments) {}
}
