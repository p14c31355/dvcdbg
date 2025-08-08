/// logger.rs
/// フォーマット付きログ用マクロ

// debug_log 有効時のみマクロ定義
#[cfg(feature = "debug_log")]
#[macro_export]
macro_rules! log {
    ($logger:expr, $($arg:tt)*) => {
        $logger.log_fmt(core::format_args!($($arg)*))
    };
}

// 無効時は空にする
#[cfg(not(feature = "debug_log"))]
#[macro_export]
macro_rules! log {
    ($logger:expr, $($arg:tt)*) => {};
}

#[cfg(feature = "debug_log")]
use core::fmt::Write;
#[cfg(feature = "debug_log")]
use heapless::String;

/// 共通の Logger トレイト（debug_log 有効時のみ）
#[cfg(feature = "debug_log")]
pub trait Logger {
    fn log(&mut self, msg: &str);
    fn log_fmt(&mut self, args: core::fmt::Arguments);

    /// バイト列を安全に 0xXX 表記でログ出力
    fn log_bytes(&mut self, label: &str, bytes: &[u8]) {
        let mut out = String::<128>::new();
        if write!(&mut out, "{label}: ").is_ok() {
            for b in bytes {
                if write!(&mut out, "0x{b:02X} ").is_err() {
                    // Buffer is full, append "..." to indicate truncation and stop.
                    let _ = out.push_str("...");
                    break;
                }
            }
        } else {
            // Label and/or separator was too long. Add ellipsis to what was written.
            let _ = out.push_str("...");
        }
        self.log(out.as_str());
    }

    fn log_i2c(&mut self, context: &str, result: Result<(), impl core::fmt::Debug>) {
        match result {
            Ok(_) => {
                let _ = self.log_fmt(format_args!("✅ {context} OK"));
            }
            Err(e) => {
                let _ = self.log_fmt(format_args!("❌ {context} FAILED: {e:?}"));
            }
        }
    }
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

    fn log_fmt(&mut self, args: core::fmt::Arguments) {
        let _ = writeln!(self.writer, "{args}");
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

    fn log_fmt(&mut self, args: core::fmt::Arguments) {
        let _ = writeln!(self.buffer, "{args}");
    }
}

/// 何も出力しないダミーロガー
pub struct NoopLogger;

impl NoopLogger {
    pub const fn new() -> Self {
        Self
    }
}

#[cfg(feature = "debug_log")]
impl Logger for NoopLogger {
    fn log(&mut self, _: &str) {}
    fn log_fmt(&mut self, _: core::fmt::Arguments) {}
}

/// 単一のコマンド値をログ出力（0xXX 表記）
#[cfg(feature = "debug_log")]
pub fn log_cmd<L: Logger>(logger: &mut L, cmd: u8) {
    logger.log_fmt(format_args!("0x{cmd:02X}"));
}