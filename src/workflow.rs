/// Quick diagnostic workflow for a new board.
///
/// Automatically performs:
/// 1. Serial logger check
/// 2. I2C bus scan
/// 3. Optional cycle measurement of a test expression
///
/// # Arguments
/// - `$serial`: Serial logger implementing `core::fmt::Write`
/// - `$i2c`: I2C bus instance
/// - `$timer`: Timer implementing `.now()`
/// - `$test_expr`: Optional expression to measure cycles for (can be `{}` block)
///
/// # Example
/// ```ignore
/// quick_diag!(logger, i2c, timer, { my_func(); });
/// ```
#[macro_export]
macro_rules! quick_diag {
    ($serial:expr, $i2c:expr, $timer:expr, $test_expr:block) => {{
        let _ = core::writeln!($serial, "=== Quick Diagnostic Start ===");

        // I2C bus scan
        let _ = core::writeln!($serial, "Scanning I2C bus...");
        $crate::scan_i2c!($i2c, $serial);

        // Test expression timing
        let (_result, cycles) = $crate::measure_cycles!($test_expr, $timer);
        let _ = core::writeln!($serial, "Test expression cycles: {}", cycles);

        let _ = core::writeln!($serial, "=== Quick Diagnostic Complete ===");
    }};
    ($serial:expr, $i2c:expr, $timer:expr) => {{
        let _ = core::writeln!($serial, "=== Quick Diagnostic Start ===");
        let _ = core::writeln!($serial, "Scanning I2C bus...");
        $crate::scan_i2c!($i2c, $serial);
        let _ = core::writeln!($serial, "=== Quick Diagnostic Complete ===");
    }};
}
