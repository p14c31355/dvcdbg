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
