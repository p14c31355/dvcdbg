//! Defines the logging level for scanner functions.
use heapless::String;
use crate::explorer::LOG_BUFFER_CAPACITY;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    /// Log verbose information, including scan progress and detailed errors.
    Verbose,
    /// Log only warnings and errors.
    Normal,
    /// Suppress all logging output.
    Quiet,
}

/// Wrapper for serial interface to implement the Logger trait
pub struct SerialLogger<'a, S: core::fmt::Write> {
    writer: &'a mut S,
    buffer: heapless::String<{ crate::explorer::LOG_BUFFER_CAPACITY }>,
    log_level: LogLevel,
}

impl<'a, S: core::fmt::Write> SerialLogger<'a, S> {
    pub fn new(writer: &'a mut S, log_level: LogLevel) -> Self {
        Self {
            writer,
            buffer: heapless::String::new(),
            log_level,
        }
    }
}

impl<'a, S: core::fmt::Write> Logger for SerialLogger<'a, S> {
    fn log_info(&mut self, msg: &str) {
        if self.log_level != LogLevel::Quiet {
            let _ = writeln!(self.writer, "[log] {msg}\r\n");
        }
    }

    fn log_warning(&mut self, msg: &str) {
        if self.log_level != LogLevel::Quiet {
            let _ = writeln!(self.writer, "[warn] {msg}\r\n");
        }
    }

    fn log_error(&mut self, msg: &str) {
        if self.log_level != LogLevel::Quiet {
            let _ = writeln!(self.writer, "[error] {msg}\r\n");
        }
    }

    fn log_info_fmt<F>(&mut self, fmt: F)
    where
        F: FnOnce(
            &mut String<{ crate::explorer::LOG_BUFFER_CAPACITY }>,
        ) -> Result<(), core::fmt::Error>,
    {
        if self.log_level != LogLevel::Quiet {
            self.buffer.clear();
            if fmt(&mut self.buffer).is_ok() {
                let _ = self.writer.write_str(self.buffer.as_str());
            }
        }
    }

    fn log_error_fmt<F>(&mut self, fmt: F)
    where
        F: FnOnce(
            &mut String<{ crate::explorer::LOG_BUFFER_CAPACITY }>,
        ) -> Result<(), core::fmt::Error>,
    {
        self.buffer.clear();
        if fmt(&mut self.buffer).is_ok() {
            let _ = self.writer.write_str(self.buffer.as_str());
        }
    }
}

/// A trait for logging progress and results.
pub trait Logger {
    fn log_info(&mut self, msg: &str);
    fn log_warning(&mut self, msg: &str);
    fn log_error(&mut self, msg: &str);

    /// Logs formatted information efficiently, by writing directly to an internal buffer.
    fn log_info_fmt<F>(&mut self, fmt: F)
    where
        F: FnOnce(&mut String<LOG_BUFFER_CAPACITY>) -> Result<(), core::fmt::Error>;

    fn log_error_fmt<F>(&mut self, fmt: F)
    where
        F: FnOnce(&mut String<LOG_BUFFER_CAPACITY>) -> Result<(), core::fmt::Error>;
}

// Dummy logger for platforms without console output
pub struct NullLogger;
impl Logger for NullLogger {
    fn log_info(&mut self, _msg: &str) {}
    fn log_warning(&mut self, _msg: &str) {}
    fn log_error(&mut self, _msg: &str) {}
    fn log_info_fmt<F>(&mut self, _fmt: F)
    where
        F: FnOnce(&mut String<LOG_BUFFER_CAPACITY>) -> Result<(), core::fmt::Error>,
    {
    }

    fn log_error_fmt<F>(&mut self, _fmt: F)
    where
        F: FnOnce(&mut String<LOG_BUFFER_CAPACITY>) -> Result<(), core::fmt::Error>,
    {
    }
}
