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
    let mut it = bytes.iter().peekable();
    while let Some(b) = it.next() {
        write_byte_hex(w, *b)?;
        if it.peek().is_some() {
            w.write_char(' ')?;
        }
    }
    Ok(())
}

pub fn write_bytes_hex_prefixed<W: Write>(w: &mut W, bytes: &[u8]) -> fmt::Result {
    let mut it = bytes.iter().peekable();
    while let Some(b) = it.next() {
        w.write_str("0x")?;
        write_byte_hex(w, *b)?;
        if it.peek().is_some() {
            w.write_char(' ')?;
        }
    }
    Ok(())
}
