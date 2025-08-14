#![no_std]

//! This crate provides utilities for fast logging in embedded environments.
//! It is no_std compatible and supports multiple logger backends.

pub mod logger;
pub mod macros;
#[cfg(feature = "debug_log")]
pub mod scanner;
