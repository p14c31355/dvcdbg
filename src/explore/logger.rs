// src/explore/logger.rs

//! Defines the logging level for scanner functions.

use crate::compat::util::ERROR_STRING_BUFFER_SIZE;
use heapless::String;

static mut LOG_BUFFER: heapless::String<ERROR_STRING_BUFFER_SIZE> = heapless::String::new();

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
        F: FnOnce(&mut String<ERROR_STRING_BUFFER_SIZE>) -> core::fmt::Result;
    fn log_error_fmt<F>(&mut self, f: F)
    where
        F: FnOnce(&mut String<ERROR_STRING_BUFFER_SIZE>) -> core::fmt::Result;
}

/// Immediate-write serial logger
pub struct SerialLogger<'a, S: core::fmt::Write> {
    writer: &'a mut S,
    log_level: LogLevel,
}

impl<'a, S> SerialLogger<'a, S>
where
    S: core::fmt::Write,
{
    pub fn new(writer: &'a mut S, log_level: LogLevel) -> Self {
        Self { writer, log_level }
    }
}

impl<'a, S> Logger for SerialLogger<'a, S>
where
    S: core::fmt::Write,
{
    fn log_info_fmt<F>(&mut self, f: F)
    where
        F: FnOnce(&mut String<ERROR_STRING_BUFFER_SIZE>) -> core::fmt::Result,
    {
        if matches!(self.log_level, LogLevel::Verbose | LogLevel::Normal) {
            unsafe {
                let log_buffer_ptr: *mut String<ERROR_STRING_BUFFER_SIZE> = &raw mut LOG_BUFFER;
                (*log_buffer_ptr).clear();

                if f(&mut *log_buffer_ptr).is_ok() {
                    let _ = self.writer.write_str("[I] ");
                    let _ = self.writer.write_str(&*log_buffer_ptr);
                    let _ = self.writer.write_str("\r\n");
                }
            }
        }
    }

    fn log_error_fmt<F>(&mut self, f: F)
    where
        F: FnOnce(&mut String<ERROR_STRING_BUFFER_SIZE>) -> core::fmt::Result,
    {
        if matches!(self.log_level, LogLevel::Verbose | LogLevel::Normal) {
            unsafe {
                let log_buffer_ptr: *mut String<ERROR_STRING_BUFFER_SIZE> = &raw mut LOG_BUFFER;
                (*log_buffer_ptr).clear();
                if f(&mut *log_buffer_ptr).is_ok() {
                    let _ = self.writer.write_str("[E] ");
                    let _ = self.writer.write_str(&*log_buffer_ptr);
                    let _ = self.writer.write_str("\r\n");
                }
            }
        }
    }
}

impl<'a, S> core::fmt::Write for SerialLogger<'a, S>
where
    S: core::fmt::Write,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.writer.write_str(s)
    }
}

pub struct NullLogger;

impl Logger for NullLogger {
    fn log_info_fmt<F>(&mut self, _f: F) {}
    fn log_error_fmt<F>(&mut self, _f: F) {}
}
