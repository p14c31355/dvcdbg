//! # dvcdbg prelude
//!
//! Common imports for ease of use.
//! Users can simply `use dvcdbg::prelude::*;` to access the main types and macros.

#[cfg(feature = "logger")]
pub use crate::logger::{
    Logger,         // Trait for logging
    SerialLogger,   // Default serial logger
    log,            // log! macro for formatted logging
};

#[cfg(feature = "macros")]
pub use crate::macros::{
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

#[cfg(feature = "quick_diag")]
pub use crate::workflow::{
    quick_diag,
};
