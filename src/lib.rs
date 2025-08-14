#![no_std]

//! # dvcdbg
//!
//! Lightweight logging and diagnostic utilities for embedded Rust.
//! Compatible with `no_std` and multiple HAL backends.

#[cfg(feature = "logger")]
pub mod logger;

#[cfg(feature = "macros")]
pub mod macros;

#[cfg(feature = "scanner")]
pub mod scanner;

#[cfg(feature = "quick_diag")]
pub mod workflow;

/// Prelude module for easy import of commonly used types and macros.
///
/// Users can simply:
/// ```rust
/// use dvcdbg::prelude::*;
/// ```
#[allow(unused_imports)]
pub mod prelude {
    #[cfg(feature = "logger")]
    pub use crate::logger::*;
    #[cfg(feature = "macros")]
    pub use crate::macros::*;
    #[cfg(feature = "scanner")]
    pub use crate::scanner::*;
    #[cfg(feature = "quick_diag")]
    pub use crate::workflow::*;
}
