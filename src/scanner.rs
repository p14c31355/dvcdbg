use crate::log;
use crate::logger::Logger;
use embedded_hal::i2c::I2c;
use heapless::Vec;

/// Scan the I2C bus for connected devices (addresses 0x03 to 0x77).
///
/// # Arguments
///
/// * `i2c` - Mutable reference to the I2C interface implementing `embedded_hal::i2c::I2c`.
/// * `logger` - Mutable reference to a logger implementing the `Logger` trait.
///
/// This function attempts to write zero bytes to each possible device address on the I2C bus,
/// logging addresses that respond successfully.
pub fn scan_i2c<I2C, L>(i2c: &mut I2C, logger: &mut L)
where
    I2C: I2c,
    L: Logger,
{
    log!(logger, "[scan] Scanning I2C bus...");
    internal_scan(i2c, logger, &[]);
    log!(logger, "[info] I2C scan complete.");
}

/// Scan the I2C bus for devices by sending specified control bytes.
///
/// # Arguments
///
/// * `i2c` - Mutable reference to the I2C interface implementing `embedded_hal::i2c::I2c`.
/// * `logger` - Mutable reference to a logger implementing the `Logger` trait.
/// * `control_bytes` - Byte slice to send as control bytes during the scan.
///
/// This function attempts to write the provided control bytes to each device address,
/// logging those that respond successfully.
pub fn scan_i2c_with_ctrl<I2C, L>(i2c: &mut I2C, logger: &mut L, control_bytes: &[u8])
where
    I2C: I2c,
    L: Logger,
{
    log!(
        logger,
        "[scan] Scanning I2C bus with control bytes: {:?}",
        control_bytes
    );
    internal_scan(i2c, logger, control_bytes);
    log!(logger, "[info] I2C scan complete.");
}

/// Scan the I2C bus by testing each command in an initialization sequence.
///
/// # Arguments
///
/// * `i2c` - Mutable reference to the I2C interface implementing `embedded_hal::i2c::I2c`.
/// * `logger` - Mutable reference to a logger implementing the `Logger` trait.
/// * `init_sequence` - Byte slice of initialization commands to test.
///
/// This function tries to send each command in `init_sequence` with a control byte (0x00)
/// to all possible device addresses, logging which addresses respond to which commands.
/// It also logs differences between expected and responding commands.
pub fn scan_init_sequence<I2C, L>(i2c: &mut I2C, logger: &mut L, init_sequence: &[u8])
where
    I2C: I2c,
    L: Logger,
{
    log!(
        logger,
        "[scan] Scanning I2C bus with init sequence: {:02X?}",
        init_sequence
    );

    let mut detected_cmds = Vec::<u8, 64>::new();

    for &cmd in init_sequence {
        log!(logger, "-> Testing command 0x{:02X}", cmd);
        internal_scan(i2c, logger, &[0x00, cmd]);

        if detected_cmds.push(cmd).is_err() {
            log!(
                logger,
                "[warn] Detected commands buffer is full, results may be incomplete!"
            );
        }
    }

    // Show differences
    log!(logger, "Expected sequence: {:02X?}", init_sequence);
    log!(
        logger,
        "Commands with response: {:02X?}",
        detected_cmds.as_slice()
    );

    detected_cmds.sort_unstable();
    let missing_cmds: Vec<u8, 64> = init_sequence
        .iter()
        .filter(|&&c| detected_cmds.binary_search(&c).is_err())
        .copied()
        .collect();

    log!(
        logger,
        "Commands with no response: {:02X?}",
        missing_cmds.as_slice()
    );

    log!(logger, "[info] I2C scan with init sequence complete.");
}

// -----------------------------------------------------------------------------
// Internal utilities (not exported as public API)
// -----------------------------------------------------------------------------

fn internal_scan<I2C, L>(i2c: &mut I2C, logger: &mut L, data: &[u8])
where
    I2C: I2c,
    L: Logger,
{
    for addr in 0x03..=0x77 {
        if i2c.write(addr, data).is_ok() {
            log!(logger, "[ok] Found device at 0x{:02X}", addr);
        }
    }
}
