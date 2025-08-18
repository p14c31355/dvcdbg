//! # dvcdbg prelude
//!
//! Common imports for ease of use.
//! Users can simply `use dvcdbg::prelude::*;` to access the main types and macros.

#[cfg(feature = "logger")]
pub use crate::{
    log,
    logger::{
        Logger,       // Trait for logging
        SerialLogger, // Default serial logger
    },
};

#[cfg(feature = "macros")]
pub use crate::{
    adapt_serial, assert_log, loop_with_delay, measure_cycles, quick_diag, write_bin, write_hex,
};

#[cfg(feature = "scanner")]
pub use crate::scanner::{scan_i2c, scan_i2c_with_ctrl, scan_init_sequence};

// Re-export functions to maintain API compatibility for macros.
#[cfg(feature = "ehal_1_0")]
pub use self::ehal_1_0::{scan_i2c, scan_i2c_with_ctrl, scan_init_sequence};

#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
pub use self::ehal_0_2::{scan_i2c, scan_i2c_with_ctrl, scan_init_sequence};