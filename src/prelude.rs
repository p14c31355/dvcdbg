//! # dvcdbg prelude
//!
//! Common imports for ease of use.
//! Users can simply `use dvcdbg::prelude::*;` to access the main types and macros.

pub use crate::{
    adapt_serial, assert_log, loop_with_delay, measure_cycles, quick_diag, write_bin, write_hex,
};

pub use crate::compat::adapt::FmtWriteAdapter;
pub use crate::compat::{HalErrorExt, I2cCompat, SerialCompat, SerialEio, UartLike};
pub use crate::explorer::{CmdExecutor, CmdNode, Explorer};
pub use crate::scanner::{scan_i2c, scan_i2c_with_ctrl, scan_init_sequence};
