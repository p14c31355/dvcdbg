use ufmt::uWrite;
use ufmt::uwriteln;

pub trait Logger {
    fn log(&mut self, msg: &str);
}

pub struct SerialLogger<'a, W: uWrite> {
    writer: &'a mut W,
}

impl<'a, W: uWrite> SerialLogger<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer }
    }
}

impl<'a, W: uWrite> Logger for SerialLogger<'a, W> {
    fn log(&mut self, msg: &str) {
        let _ = uwriteln!(self.writer, "{}", msg);
    }
}

pub struct NoopLogger;

impl Logger for NoopLogger {
    fn log(&mut self, _: &str) {}
}
