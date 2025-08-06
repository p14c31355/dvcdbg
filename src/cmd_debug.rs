use heapless::String;
use crate::logger::Logger;

#[cfg(test)]
use sh1107g_rs::cmds::*;

#[cfg(test)]
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
fn byte_to_hex(byte: u8) -> String<6> {
    use core::fmt::Write;
    let mut s = String::<6>::new();
    let _ = write!(s, "0x{:02X}", byte);
    s
}

/*
checklist

Add to Sh1107g logger
Implementation new_with_logger()
Implementation log call method to send_cmd()
Config attribute
*/