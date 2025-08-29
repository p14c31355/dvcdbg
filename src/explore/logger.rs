//! Defines the logging level for scanner functions.

use core::fmt; // Add this line

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Verbose,
    Normal,
    Quiet,
}

impl<'a, S: core::fmt::Write, const B: usize> core::fmt::Write for SerialLogger<'a, S, B> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.writer.write_str(s)
    }
}

/// Trait for logging progress and results.
pub trait Logger<const B: usize> {
    fn log_info_fmt<F>(&mut self, f: F)
    where
        F: FnOnce(&mut heapless::String<B>) -> core::fmt::Result;
    fn log_error_fmt<F>(&mut self, f: F)
    where
        F: FnOnce(&mut heapless::String<B>) -> core::fmt::Result;
}

/// Immediate-write serial logger, writes directly to the underlying serial interface.
pub struct SerialLogger<'a, S: core::fmt::Write, const B: usize> {
    writer: &'a mut S,
    buffer: heapless::String<{ B }>,
    log_level: LogLevel,
}

impl<'a, S: core::fmt::Write, const B: usize> SerialLogger<'a, S, B> {
    pub fn new(writer: &'a mut S, log_level: LogLevel) -> Self {
        Self {
            writer,
            buffer: heapless::String::new(),
            log_level,
        }
    }
}

impl<'a, S, const B: usize> Logger<B> for SerialLogger<'a, S, B>
where
    S: core::fmt::Write,
{
    fn log_info_fmt<F>(&mut self, f: F)
    where
        F: FnOnce(&mut heapless::String<B>) -> core::fmt::Result,
    {
        if self.log_level != LogLevel::Quiet {
            self.buffer.clear();
            if f(&mut self.buffer).is_ok() {
                writeln!(self.writer, "{}", self.buffer).ok();
            }
        }
    }

    fn log_error_fmt<F>(&mut self, f: F)
    where
        F: FnOnce(&mut heapless::String<B>) -> core::fmt::Result,
    {
        if self.log_level != LogLevel::Quiet {
            self.buffer.clear();
            if f(&mut self.buffer).is_ok() {
                writeln!(self.writer, "{}", self.buffer).ok();
            }
        }
    }
}

/// A trait for platforms without console output.
pub struct NullLogger;
impl<const B: usize> Logger<B> for NullLogger {
    fn log_info_fmt<F>(&mut self, _fmt: F)
    where
        F: FnOnce(&mut heapless::String<B>) -> Result<(), core::fmt::Error>,
    {
    }
    fn log_error_fmt<F>(&mut self, _fmt: F)
    where
        F: FnOnce(&mut heapless::String<B>) -> Result<(), core::fmt::Error>,
    {
    }
}

/// Writes a slice of bytes as a hexadecimal string using an internal buffer.
///
/// This function is useful when you need to format bytes within a larger `writeln!` macro
/// and want to avoid intermediate string allocations.
///
/// # Arguments
/// * `serial` - A mutable reference to a type that implements `core::fmt::Write`.
/// * `bytes` - The slice of bytes to format.
///
/// # Example
/// ```
/// use heapless::String;
/// use core::fmt::Write;
/// use dvcdbg::explore::logger::write_bytes_hex_buffered;
///
/// let mut s: String<64> = String::new();
/// let bytes = [0xDE, 0xAD, 0xBE, 0xEF];
///
/// writeln!(s, "Data: ").unwrap();
/// write_bytes_hex_buffered(&mut s, &bytes).unwrap(); // Renamed function
/// assert_eq!(s.as_str(), "Data: DEADBEFF");
/// ```
pub fn write_bytes_hex_buffered<S, const BUF_CAP: usize>(
    serial: &mut S,
    bytes: &[u8],
) -> Result<(), core::fmt::Error>
where
    S: core::fmt::Write,
{
    let mut temp_string: heapless::String<BUF_CAP> = heapless::String::new();
    for byte in bytes {
        fmt::Write::write_fmt(&mut temp_string, core::format_args!("{:02X}", byte))?;
    }
    write!(serial, "{}", temp_string)
}
