#![no_std]

//! # dvcdbg
//!
//! Lightweight logging and diagnostic utilities for embedded Rust.
//! Compatible with `no_std` and multiple HAL backends.

#[cfg(feature = "logger")]
pub mod logger;

#[cfg(feature = "scanner")]
pub mod scanner;

#[cfg(feature = "macros")]
#[macro_use]
pub mod macros;

/// Prelude module for easy import of commonly used types and macros.
///
/// Users can simply:
/// ```rust
/// use dvcdbg::prelude::*;
/// ```
pub mod prelude;

/// Recursive log macro that enables log output within macros.
///
/// Formats arguments into a temporary `heapless::String` of a fixed size (128 bytes).
/// If the formatted output exceeds the buffer capacity, it will be silently truncated.
/// This is useful for preparing a string to be passed to another logging macro.
#[macro_export]
macro_rules! recursive_log {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut buf: heapless::String<128> = heapless::String::new();
        let _ = write!(buf, $($arg)*);
        buf
    }};
}

/// Error type returned by [`adapt_serial!`] adapters.
///
/// This type is part of the public API, but its exact variants may change
/// in a minor release. Prefer matching with `_` to stay forward-compatible.
///  
/// This is public because wiring issues are common in prototyping,
/// and users may want to handle them (e.g., retries, logging).
#[derive(Debug)]
pub enum AdaptError<E> {
    /// Formatting failure (e.g., `core::fmt::Write`).
    /// This variant is not currently used,
    /// but is reserved for compatibility absorption in the event of future disruptive changes.
    Fmt,
    /// HAL-specific error.
    Other(E),
}

impl<E: core::fmt::Debug> embedded_io::Error for AdaptError<E> {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}
