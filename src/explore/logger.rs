// src/explore/logger.rs

//! Defines the logging level for scanner functions.

use heapless::String;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Verbose,
    Normal,
    Quiet,
}

/// Trait for logging progress and results.
pub trait Logger {
    fn log_info_fmt<F>(&mut self, f: F)
    where
        F: FnOnce(&mut String<512>) -> core::fmt::Result;
    fn log_error_fmt<F>(&mut self, f: F)
    where
        F: FnOnce(&mut String<512>) -> core::fmt::Result;
}

/// Immediate-write serial logger, writes directly to the underlying serial interface.
pub struct SerialLogger<'a, S: core::fmt::Write> {
    writer: &'a mut S,
    buffer: String<512>,
    log_level: LogLevel,
}

impl<'a, S: core::fmt::Write> SerialLogger<'a, S> {
    pub fn new(writer: &'a mut S, log_level: LogLevel) -> Self {
        Self {
            writer,
            buffer: String::new(),
            log_level,
        }
    }
}

impl<'a, S: core::fmt::Write> Logger for SerialLogger<'a, S> {
    fn log_info_fmt<F>(&mut self, f: F)
    where
        F: FnOnce(&mut String<512>) -> core::fmt::Result,
    {
        if self.log_level == LogLevel::Verbose || self.log_level == LogLevel::Normal {
            self.buffer.clear();
            if f(&mut self.buffer).is_ok() {
                self.writer.write_str(self.buffer.as_str()).ok();
            }
        }
    }

    fn log_error_fmt<F>(&mut self, f: F)
    where
        F: FnOnce(&mut String<512>) -> core::fmt::Result,
    {
        if self.log_level == LogLevel::Verbose || self.log_level == LogLevel::Normal {
            self.buffer.clear();
            if f(&mut self.buffer).is_ok() {
                self.writer.write_str(self.buffer.as_str()).ok();
            }
        }
    }
}

impl<'a, S: core::fmt::Write> core::fmt::Write for SerialLogger<'a, S> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.writer.write_str(s)
    }
}

/// A trait for platforms without console output.
pub struct NullLogger;

impl Logger for NullLogger {
    fn log_info_fmt<F>(&mut self, _fmt: F)
    where
        F: FnOnce(&mut String<512>) -> Result<(), core::fmt::Error>,
    {
    }
    fn log_error_fmt<F>(&mut self, _fmt: F)
    where
        F: FnOnce(&mut String<512>) -> Result<(), core::fmt::Error>,
    {
    }
}
