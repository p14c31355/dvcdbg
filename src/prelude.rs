//! # dvcdbg prelude
//!
//! Common imports for ease of use.
//! Users can simply `use dvcdbg::prelude::*;` to access the main types and macros.

pub use crate::{
    adapt_serial,
    assert_log,
    loop_with_delay,
    measure_cycles,
    quick_diag,
    write_bin,
    write_hex,
};

// ユーザー向けAPIだけ export
pub use crate::scanner::{scan_i2c, scan_i2c_with_ctrl, scan_init_sequence};
pub use crate::compat::i2c_compat::I2cCompat;
pub use crate::compat::serial_compat::SerialCompat;
