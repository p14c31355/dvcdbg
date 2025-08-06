use crate::logger::Logger;

pub enum Command {
    DisplayOn,
    DisplayOff,
    SetStartLine(u8),
    Unknown(u8),
}

impl Command {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0xAF => Command::DisplayOn,
            0xAE => Command::DisplayOff,
            b if b & 0xC0 == 0x40 => Command::SetStartLine(b & 0x3F),
            _ => Command::Unknown(byte),
        }
    }

    pub fn log<L: Logger>(&self, logger: &mut L) {
        match self {
            Command::DisplayOn => logger.log("Display ON"),
            Command::DisplayOff => logger.log("Display OFF"),
            Command::SetStartLine(val) => {
                let mut buf = [0u8; 32];
                let mut writer = ufmt::uWriteBuf::new(&mut buf);
                let _ = ufmt::uwriteln!(&mut writer, "Set Start Line: {}", val);
                logger.log(core::str::from_utf8(writer.as_slice()).unwrap_or("UTF-8 error"));
            }
            Command::Unknown(code) => {
                let mut buf = [0u8; 32];
                let mut writer = ufmt::uWriteBuf::new(&mut buf);
                let _ = ufmt::uwriteln!(&mut writer, "Unknown Command: 0x{:02X}", code);
                logger.log(core::str::from_utf8(writer.as_slice()).unwrap_or("UTF-8 error"));
            }
        }
    }
}

use sh1107g_rs::cmds::*;
use crate::logger::Logger;

/// デバッグ用の初期化コマンド列をログ出力する
pub fn log_init_sequence<L: Logger>(logger: &mut L) {
    log_cmd(logger, DISPLAY_OFF);
    log_cmd(logger, SET_MULTIPLEX_RATIO);
    log_cmd(logger, MULTIPLEX_RATIO_DATA);
    log_cmd(logger, CHARGE_PUMP_ON_CMD);
    log_cmd(logger, CHARGE_PUMP_ON_DATA);
    log_cmd(logger, PAGE_ADDRESSING_CMD);
    log_cmd(logger, SEGMENT_REMAP);
    log_cmd(logger, COM_OUTPUT_SCAN_DIR);
    log_cmd(logger, DISPLAY_START_LINE_CMD);
    log_cmd(logger, DISPLAY_START_LINE_DATA);
    log_cmd(logger, CONTRAST_CONTROL_CMD);
    log_cmd(logger, CONTRAST_CONTROL_DATA);
    log_cmd(logger, DISPLAY_OFFSET_CMD);
    log_cmd(logger, DISPLAY_OFFSET_DATA);
    log_cmd(logger, PRECHARGE_CMD);
    log_cmd(logger, PRECHARGE_DATA);
    log_cmd(logger, VCOM_DESELECT_CMD);
    log_cmd(logger, VCOM_DESELECT_DATA);
    log_cmd(logger, CLOCK_DIVIDE_CMD);
    log_cmd(logger, CLOCK_DIVIDE_DATA);
    log_cmd(logger, SET_COM_PINS_CMD);
    log_cmd(logger, SET_COM_PINS_DATA);
    log_cmd(logger, SET_ENTIRE_DISPLAY_ON_OFF_CMD);
    log_cmd(logger, SET_NORMAL_INVERSE_DISPLAY_CMD);
    log_cmd(logger, DISPLAY_ON);
}

/// 任意のコマンド値をログ出力する
fn log_cmd<L: Logger>(logger: &mut L, cmd: u8) {
    let hex = byte_to_hex(cmd);
    logger.log(&hex);
}

/// u8 を `"0xXX"` 形式の16進文字列に変換
fn byte_to_hex(byte: u8) -> heapless::String<6> {
    use core::fmt::Write;
    let mut s = heapless::String::<6>::new();
    let _ = write!(s, "0x{:02X}", byte);
    s
}

