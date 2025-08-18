use crate::log;
use crate::logger::Logger;
use embedded_hal::i2c::I2c;
use heapless::Vec;

// -----------------------------------------------------------------------------
//  Public API (with Rustdoc) 
// -----------------------------------------------------------------------------

/// Scan the I2C bus for connected devices (addresses `0x03` to `0x77`).
///
/// This function probes each possible I2C device address by attempting to
/// write an empty buffer (`[]`). Devices that acknowledge are reported
/// through the provided logger.
///
/// # Arguments
///
/// * `i2c` - Mutable reference to the I2C interface implementing [`embedded_hal::i2c::I2c`].
/// * `logger` - Mutable reference to a logger implementing the [`Logger`] trait.
///
/// # Example
///
/// ```ignore
/// use embedded_hal::i2c::I2c;
/// use dvcdbg::logger::SerialLogger;
/// use dvcdbg::scanner::scan_i2c;
///
/// let mut i2c = /* your i2c interface */;
/// let mut logger = SerialLogger::new(/* serial */);
///
/// scan_i2c(&mut i2c, &mut logger);
/// ```
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
/// This variant allows specifying control bytes (e.g., `0x00`) to send
/// alongside the probe. Devices that acknowledge the transmission are
/// reported via the logger.
///
/// # Arguments
///
/// * `i2c` - Mutable reference to the I2C interface implementing [`embedded_hal::i2c::I2c`].
/// * `logger` - Mutable reference to a logger implementing the [`Logger`] trait.
/// * `control_bytes` - Slice of bytes to send when probing each device.
///
/// # Example
///
/// ```ignore
/// use embedded_hal::i2c::I2c;
/// use dvcdbg::logger::SerialLogger;
/// use dvcdbg::scanner::scan_i2c_with_ctrl;
///
/// let mut i2c = /* your i2c interface */;
/// let mut logger = SerialLogger::new(/* serial */);
///
/// scan_i2c_with_ctrl(&mut i2c, &mut logger, &[0x00]);
/// ```
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

/// Scan the I2C bus using an initialization sequence of commands.
///
/// Each command in the sequence is transmitted to all possible device
/// addresses using the control byte `0x00`. The function records which
/// commands receive responses and highlights any **differences** between
/// the expected and observed responses.
///
/// This is useful for verifying whether a device supports the expected
/// initialization commands (e.g., when testing display controllers).
///
/// # Arguments
///
/// * `i2c` - Mutable reference to the I2C interface implementing [`embedded_hal::i2c::I2c`].
/// * `logger` - Mutable reference to a logger implementing the [`Logger`] trait.
/// * `init_sequence` - Slice of initialization commands to test.
///
/// # Example
///
/// ```ignore
/// use embedded_hal::i2c::I2c;
/// use dvcdbg::logger::SerialLogger;
/// use dvcdbg::scanner::scan_init_sequence;
///
/// let mut i2c = /* your i2c interface */;
/// let mut logger = SerialLogger::new(/* serial */);
///
/// let init_sequence: [u8; 3] = [0xAE, 0xA1, 0xAF]; // example init cmds
/// scan_init_sequence(&mut i2c, &mut logger, &init_sequence);
/// ```
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

    log_differences(logger, init_sequence, &detected_cmds);
    log!(logger, "[info] I2C scan with init sequence complete.");
}

// -----------------------------------------------------------------------------
//  Internal utilities (not exported as public API) 
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

fn log_differences<L>(logger: &mut L, expected: &[u8], detected: &Vec<u8, 64>)
where
    L: Logger,
{
    log!(logger, "Expected sequence: {:02X?}", expected);
    log!(logger, "Commands with response: {:02X?}", detected.as_slice());

    let mut sorted = detected.clone();
    sorted.sort_unstable();
    let missing_cmds: Vec<u8, 64> = expected
        .iter()
        .filter(|&&c| sorted.binary_search(&c).is_err())
        .copied()
        .collect();

    log!(
        logger,
        "Commands with no response: {:02X?}",
        missing_cmds.as_slice()
    );
}
