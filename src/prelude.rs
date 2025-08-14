//! # dvcdbg prelude
//!
//! Common imports for ease of use.
//! Users can simply `use dvcdbg::prelude::*;` to access the main types and macros.

#[cfg(feature = "logger")]
pub use crate::{log, logger::{
    Logger,         // Trait for logging
    SerialLogger,   // Default serial logger
}};

#[cfg(feature = "macros")]
pub use crate::{
    impl_fmt_write_for_serial,
    write_hex,
    write_bin,
    measure_cycles,
    loop_with_delay,
    assert_log,
};

#[cfg(feature = "scanner")]
pub use crate::scanner::{
    scan_i2c,
    scan_i2c_with_ctrl,
};
