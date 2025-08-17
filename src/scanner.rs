//! scanner.rs
//! I2C デバイススキャンユーティリティ

use crate::logger::log; // log! マクロ
use crate::logger::Logger;

#[cfg(feature = "ehal_0_2")]
use embedded_hal::blocking::i2c::Write as I2cWrite;

#[cfg(feature = "ehal_1_0")]
use embedded_hal_1::i2c::I2c;

/// I2C スキャン（バージョンごとに分ける）
///
/// control_bytes: 書き込み時に送る任意バイト列
/// init_sequence: 初期化コマンド列
#[cfg(feature = "ehal_0_2")]
pub fn scan_i2c_inner<I2C, L>(
    i2c: &mut I2C,
    logger: &mut L,
    control_bytes: Option<&[u8]>,
    init_sequence: Option<&[u8]>,
) where
    I2C: I2cWrite<u8>,
    L: Logger,
{
    let mut write_fn = |i2c: &mut I2C, addr: u8, data: &[u8]| -> bool {
        i2c.write(addr, data).is_ok()
    };

    scan_logic(i2c, logger, control_bytes, init_sequence, &mut write_fn);
}

#[cfg(feature = "ehal_1_0")]
pub fn scan_i2c_inner<I2C, L>(
    i2c: &mut I2C,
    logger: &mut L,
    control_bytes: Option<&[u8]>,
    init_sequence: Option<&[u8]>,
) where
    I2C: I2c,
    I2C::Error: core::fmt::Debug,
    L: Logger,
{
    let mut write_fn = |i2c: &mut I2C, addr: u8, data: &[u8]| -> bool {
        I2c::write(i2c, addr, data).is_ok()
    };

    scan_logic(i2c, logger, control_bytes, init_sequence, &mut write_fn);
}

/// 共通スキャン処理
fn scan_logic<I2C, L>(
    i2c: &mut I2C,
    logger: &mut L,
    control_bytes: Option<&[u8]>,
    init_sequence: Option<&[u8]>,
    write_fn: &mut dyn FnMut(&mut I2C, u8, &[u8]) -> bool,
) where
    L: Logger,
{
    if let Some(seq) = init_sequence {
        for &cmd in seq {
            for addr in 0x03..=0x77 {
                if write_fn(i2c, addr, &[0x00, cmd]) {
                    log!(logger, "[ok] Found device at 0x{:02X} responding to 0x{:02X}", addr, cmd);
                }
            }
        }
        log!(logger, "[info] I2C scan with init sequence complete.");
        return;
    }

    let ctrl = control_bytes.unwrap_or(&[]);
    for addr in 0x03..=0x77 {
        if write_fn(i2c, addr, ctrl) {
            log!(logger, "[ok] Found device at 0x{:02X}", addr);
        }
    }
    log!(logger, "[info] I2C scan complete.");
}

/// 上位関数：シンプルな I2C スキャン
pub fn scan_i2c<I2C, L>(i2c: &mut I2C, logger: &mut L)
where
    L: Logger,
    I2C: Sized,
{
    scan_i2c_inner(i2c, logger, None, None);
}

/// 上位関数：コントロールバイト付きスキャン
pub fn scan_i2c_with_ctrl<I2C, L>(i2c: &mut I2C, logger: &mut L, control_bytes: &[u8])
where
    L: Logger,
    I2C: Sized,
{
    scan_i2c_inner(i2c, logger, Some(control_bytes), None);
}

/// 上位関数：初期化コマンド列付きスキャン
pub fn scan_init_sequence<I2C, L>(i2c: &mut I2C, logger: &mut L, init_sequence: &[u8])
where
    L: Logger,
    I2C: Sized,
{
    scan_i2c_inner(i2c, logger, None, Some(init_sequence));
}
