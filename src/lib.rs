#![no_std]

//! # dvcdbg
//!
//! Lightweight diagnostic utilities for embedded Rust.
//! Compatible with `no_std` and multiple HAL backends.

pub mod scanner;

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
pub mod error;

