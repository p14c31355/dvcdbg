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
