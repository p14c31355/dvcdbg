// logger.rs

use ufmt::uWrite;
use ufmt::uwriteln;

/// ログ出力インタフェース（任意の出力先に対応）
pub trait Logger {
    fn log(&mut self, msg: &str);
}

/// UARTなどに出力するロガー
pub struct SerialLogger<W: uWrite> {
    writer: W,
}

impl<W: uWrite> SerialLogger<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }
}

impl<W: uWrite> Logger for SerialLogger<W> {
    fn log(&mut self, msg: &str) {
        let _ = uwriteln!(self.writer, "{}", msg);
    }
}

/// ログ出力を無効化するダミーロガー
pub struct NoopLogger;

impl Logger for NoopLogger {
    fn log(&mut self, _: &str) {}
}
