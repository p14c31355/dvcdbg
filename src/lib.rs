#![no_std]

//! This crate provides utilities for fast logging in embedded environments.
//! It is no_std compatible and supports multiple logger backends.

#[cfg(feature = "logger")]
pub mod logger;

#[cfg(feature = "macros")]
pub mod macros;

#[cfg(feature = "scanner")]
pub mod scanner;

#[cfg(feature = "quick_diag")]
pub mod workflow;

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
