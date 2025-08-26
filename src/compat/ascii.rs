//! ascii.rs

use core::fmt::{self, Write};

pub fn write_byte_hex<W: Write>(w: &mut W, byte: u8) -> fmt::Result {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let hi = HEX[(byte >> 4) as usize] as char;
    let lo = HEX[(byte & 0x0F) as usize] as char;
    w.write_char(hi)?;
    w.write_char(lo)?;
    Ok(())
}

pub fn write_bytes_hex<W: Write>(w: &mut W, bytes: &[u8]) -> fmt::Result {
    for (i, &b) in bytes.iter().enumerate() {
        write_byte_hex(w, b)?;
        if i != bytes.len() - 1 {
            w.write_char(' ')?;
        }
    }
    Ok(())
}

pub fn write_bytes_hex_prefixed<W: Write>(w: &mut W, bytes: &[u8]) -> fmt::Result {
    for &b in bytes {
        w.write_str("0x")?;
        write_byte_hex(w, b)?;
        w.write_char(' ')?;
    }
    Ok(())
}
