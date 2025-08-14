/// Macro that implements core::fmt::Write for any serial type
/// 
/// # Arguments
/// - `$type`: Target type (eg: `arduino_hal::DefaultSerial`)
/// - `$write_method`: 1-byte send method (eg: `write_byte`, `write`)
///
/// # Example
/// ```ignore
/// impl_fmt_write_for_serial!(arduino_hal::DefaultSerial, write_byte);
/// impl_fmt_write_for_serial!(esp_idf_hal::uart::UartDriver, write);
/// ```
#[macro_export]
macro_rules! impl_fmt_write_for_serial {
    ($type:ty, $write_method:ident) => {
        impl core::fmt::Write for $type {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for &b in s.as_bytes() {
                    // Ignore writing error (If need, add handling)
                    let _ = self.$write_method(b);
                }
                Ok(())
            }
        }
    };
}
