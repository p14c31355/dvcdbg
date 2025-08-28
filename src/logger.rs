//! Defines the logging level for scanner functions.

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
    fn log_info(&mut self, msg: &str);
    fn log_warning(&mut self, msg: &str);
    fn log_error(&mut self, msg: &str);

    fn log_info_fmt<F>(&mut self, fmt: F)
    where
        F: FnOnce(&mut heapless::String<B>) -> Result<(), core::fmt::Error>;

    fn log_error_fmt<F>(&mut self, fmt: F)
    where
        F: FnOnce(&mut heapless::String<B>) -> Result<(), core::fmt::Error>;
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

impl<'a, S: core::fmt::Write, const B: usize> Logger<B> for SerialLogger<'a, S, B> {
    fn log_info(&mut self, msg: &str) {
        if self.log_level != LogLevel::Quiet {
            let _ = write!(self.writer, "[Info] {msg}\r\n");
        }
    }

    fn log_warning(&mut self, msg: &str) {
        if self.log_level != LogLevel::Quiet {
            let _ = write!(self.writer, "[Warn] {msg}\r\n");
        }
    }

    fn log_error(&mut self, msg: &str) {
        if self.log_level != LogLevel::Quiet {
            let _ = write!(self.writer, "[Error] {msg}\r\n");
        }
    }

    fn log_info_fmt<F>(&mut self, fmt: F)
    where
        F: FnOnce(&mut heapless::String<B>) -> Result<(), core::fmt::Error>,
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
        F: FnOnce(&mut heapless::String<B>) -> Result<(), core::fmt::Error>,
    {
        if self.log_level != LogLevel::Quiet {
            self.buffer.clear();
            if fmt(&mut self.buffer).is_ok() {
                let _ = self.writer.write_str(self.buffer.as_str());
            }
        }
    }
}

/// A trait for platforms without console output.
pub struct NullLogger;
impl<const B: usize> Logger<B> for NullLogger {
    fn log_info(&mut self, _msg: &str) {}
    fn log_warning(&mut self, _msg: &str) {}
    fn log_error(&mut self, _msg: &str) {}
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
