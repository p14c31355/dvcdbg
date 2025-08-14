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

/// Writes a byte slice in hex format to a `fmt::Write` target.
#[macro_export]
macro_rules! write_hex {
    ($dst:expr, $data:expr) => {
        for &b in $data {
            let _ = core::write!($dst, "{:02X} ", b);
        }
    };
}

macro_rules! write_bin {
    ($dst:expr, $data:expr) => {
        for &b in $data {
            let _ = core::write!($dst, "{:08b} ", b);
        }
    };
}

#[macro_export]
macro_rules! measure_cycles {
    ($expr:expr, $timer:expr) => {{
        let start = $timer.now();
        let result = $expr;
        let elapsed = $timer.now().wrapping_sub(start);
        (result, elapsed)
    }};
}

#[macro_export]
macro_rules! loop_with_delay {
    ($delay:expr, $body:block) => {
        loop {
            $body
            $delay.delay_ms(1000u32);
        }
    };
}

#[macro_export]
macro_rules! assert_log {
    ($cond:expr, $logger:expr, $($arg:tt)*) => {
        if !$cond {
            let _ = core::write!($logger, "ASSERT FAILED: ");
            let _ = core::writeln!($logger, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! scan_i2c {
    ($i2c:expr, $logger:expr) => {{
        for addr in 0x03..0x78 {
            if $i2c.write(addr, &[]).is_ok() {
                let _ = core::writeln!($logger, "Found: 0x{:02X}", addr);
            }
        }
    }};
}