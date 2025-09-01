//! # dvcdbg prelude
//!
//! Common imports for ease of use.
//! Users can simply `use dvcdbg::prelude::*;` to access the main types and macros.

pub use crate::{
    adapt_serial, assert_log, factorial_sort, get_one_sort, loop_with_delay, measure_cycles,
    pruning_sort, quick_diag, write_bin, write_hex,
};

pub use crate::compat::adapt::FmtWriteAdapter;
pub use crate::compat::util::{write_byte_hex, write_bytes_hex, write_bytes_hex_prefixed};
pub use crate::compat::{HalErrorExt, I2cCompat, SerialCompat, SerialEio, UartLike};
pub use crate::error::ExecutorError;
pub use crate::explore::explorer::{CmdExecutor, CmdNode, Explorer};

pub use crate::scanner::{scan_i2c, scan_init_sequence};
