//! compat/mod.rs
pub mod adapt;
pub mod ascii;
pub mod err_compat;
pub mod i2c_compat;
pub mod serial_compat;

pub use adapt::FmtWriteAdapter;
pub use err_compat::HalErrorExt;
pub use i2c_compat::I2cCompat;
pub use serial_compat::{SerialCompat, SerialEio, UartLike};
