//! Re-exports common types and macros for convenient access.
//! Users can simply `use dvcdbg::prelude::*;` to access the main types and macros.

pub use crate::{
    adapt_serial, assert_log, get_one_sort, loop_with_delay, measure_cycles, nodes, pruning_sort,
    quick_diag, write_bin, write_hex,
};

pub use crate::compat::adapt::FmtWriteAdapter;
pub use crate::compat::err_compat::HalErrorExt;
pub use crate::compat::i2c_compat::I2cCompat;
pub use crate::compat::serial_compat::SerialCompat;
pub use crate::error::{BufferError, ErrorKind, ExecutorError, ExplorerError, I2cError, UartError};
pub use crate::scanner::{scan_i2c, scan_init_sequence};