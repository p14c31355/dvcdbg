//! compat/mod.rs
pub mod adapt;
pub mod i2c_compat;
pub mod serial_compat;
pub mod err_compat;

pub use i2c_compat::I2cCompat;
pub use serial_compat::SerialCompat;
pub use err_compat::HalErrorExt;