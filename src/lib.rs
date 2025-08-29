#![no_std]

//! # dvcdbg
//!
//! Lightweight diagnostic utilities for embedded Rust.
//! Compatible with `no_std` and multiple HAL backends.

pub mod scanner;

#[macro_use]
pub mod macros;

pub mod compat;
pub mod explore;
pub mod error;
pub mod prelude;
