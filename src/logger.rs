/// フォーマット付きログ用マクロ
#[macro_export]
macro_rules! log {
    ($logger:expr, $($arg:tt)*) => {
        $logger.log_fmt(core::format_args!($($arg)*))
    };
}

// logger.rs

#[cfg(feature = "debug_log")]
use core::fmt::Write;

#[cfg(feature = "debug_log")]
use heapless::String;

/// 共通の Logger トレイト（debug_log 有効時のみ）
#[cfg(feature = "debug_log")]
pub trait Logger {
    fn log(&mut self, msg: &str);
}

/// シリアル出力用ロガー（fmt::Write 対応機器向け）
#[cfg(feature = "debug_log")]
pub struct SerialLogger<'a, W: Write> {
    writer: &'a mut W,
}

#[cfg(feature = "debug_log")]
impl<'a, W: Write> SerialLogger<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer }
    }

    pub fn writer_mut(&mut self) -> &mut W {
        self.writer
    }
}

#[cfg(feature = "debug_log")]
impl<'a, W: Write> Logger for SerialLogger<'a, W> {
    fn log(&mut self, msg: &str) {
        let _ = writeln!(self.writer, "{msg}");
    }
}

/// バッファにログを蓄積するロガー（heapless::String）
#[cfg(feature = "debug_log")]
pub struct BufferedLogger<const N: usize> {
    buffer: String<N>,
}

#[cfg(feature = "debug_log")]
impl<const N: usize> Default for BufferedLogger<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "debug_log")]
impl<const N: usize> BufferedLogger<N> {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(feature = "debug_log")]
impl<const N: usize> Logger for BufferedLogger<N> {
    fn log(&mut self, msg: &str) {
        let _ = writeln!(self.buffer, "{msg}");
    }
}

/// 何も出力しないダミーロガー（debug_log 無効時）
#[cfg(not(feature = "debug_log"))]
pub struct NoopLogger;

#[cfg(not(feature = "debug_log"))]
impl NoopLogger {
    pub fn new() -> Self {
        Self
    }
}

/// 任意のコマンド値をログ出力する
#[cfg(feature = "debug_log")]
fn log_cmd<L: Logger>(logger: &mut L, cmd: u8) {
    let hex = byte_to_hex(cmd);
    logger.log(&hex);
}

/// u8 を `"0xXX"` 形式の16進文字列に変換
fn byte_to_hex(byte: u8) -> String<6> {
    use core::fmt::Write;
    let mut s = String::<6>::new();
    let _ = write!(s, "0x{byte:02X}");
    s
}