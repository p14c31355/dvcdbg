// logger.rs

use heapless::{Vec, String};

/// ログ出力インタフェース（任意の出力先に対応）
pub trait Logger {
    fn log(&mut self, msg: &str);
}

/// UARTなどに出力するロガー
pub struct SerialLogger<const N: usize> {
    buf: Vec<u8, N>,
    writer: String<N>,
}

impl<const N: usize> SerialLogger<N> {
    pub fn new(writer: String<N>) -> Self {
        Self {
            buf: Vec::new(),
            writer,
        }
    }

    pub fn writer_mut(&mut self) -> &mut String<N> {
        &mut self.writer
    }
}

impl<const N: usize> Logger for SerialLogger<N> {
    fn log(&mut self, msg: &str) {
        let _ = self.buf.extend_from_slice(msg.as_bytes());
    }
}

/// ログ出力を無効化するダミーロガー
pub struct NoopLogger;

impl Logger for NoopLogger {
    fn log(&mut self, _: &str) {}
}
