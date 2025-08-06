use core::fmt::{self, Write};
use heapless::String;

/// ログ出力インターフェース（任意の出力先に対応）
pub trait Logger {
    fn log(&mut self, msg: &str);
    fn log_fmt(&mut self, args: fmt::Arguments) {
        let mut buf: String<128> = String::new();
        let _ = buf.write_fmt(args);
        self.log(&buf);
    }
}

/// フォーマット付きログ用マクロ
#[macro_export]
macro_rules! log {
    ($logger:expr, $($arg:tt)*) => {
        $logger.log_fmt(core::format_args!($($arg)*))
    };
}

/// UARTなどに出力するロガー（write_str() を実装する対象に書き込む）
pub struct SerialLogger<'a, W: Write> {
    writer: &'a mut W,
}

impl<'a, W: Write> SerialLogger<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer }
    }

    pub fn writer_mut(&mut self) -> &mut W {
        self.writer
    }
}

impl<'a, W: Write> Logger for SerialLogger<'a, W> {
    fn log(&mut self, msg: &str) {
        let _ = writeln!(self.writer, "{}", msg);
    }
}

/// ログ出力を無効化するダミーロガー
pub struct NoopLogger;

impl Logger for NoopLogger {
    fn log(&mut self, _: &str) {}
}
