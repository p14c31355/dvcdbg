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

pub mod compat;

/// Recursive log macro that enables log output within macros.
///
/// Formats arguments into a temporary `heapless::String` of a fixed size (128 bytes).
/// If the formatted output exceeds the buffer capacity, it will be silently truncated.
/// This is useful for preparing a string to be passed to another logging macro.
#[macro_export]
macro_rules! recursive_log {
    ($($arg:tt)*) => {{
        const RECURSIVE_LOG_BUF_SIZE: usize = 128;
        use core::fmt::Write;
        let mut buf: heapless::String<RECURSIVE_LOG_BUF_SIZE> = heapless::String::new();
        let _ = write!(buf, $($arg)*);
        buf
    }};
}
