// src/compat/util.rs

use crate::error::BitFlagsError;

/// A bitflag structure optimized for 128 bits, used for tracking I2C addresses.
pub struct BitFlags {
    bytes: [u8; 16],
}

impl Default for BitFlags {
    fn default() -> Self {
        Self::new()
    }
}

impl BitFlags {
    pub const fn new() -> Self {
        Self { bytes: [0u8; 16] }
    }

    const N_BITS: usize = 128;

    fn check_bounds(&self, idx: usize) -> Result<(), BitFlagsError> {
        if idx >= Self::N_BITS {
            Err(BitFlagsError::IndexOutOfBounds {
                idx,
                max: Self::N_BITS - 1,
            })
        } else {
            Ok(())
        }
    }

    pub fn set(&mut self, idx: usize) -> Result<(), BitFlagsError> {
        self.check_bounds(idx)?;
        let byte = idx / 8;
        let bit = idx % 8;
        self.bytes[byte] |= 1 << bit;
        Ok(())
    }

    pub fn clear(&mut self, idx: usize) -> Result<(), BitFlagsError> {
        self.check_bounds(idx)?;
        let byte = idx / 8;
        let bit = idx % 8;
        self.bytes[byte] &= !(1 << bit);
        Ok(())
    }

    pub fn get(&self, idx: usize) -> Result<bool, BitFlagsError> {
        self.check_bounds(idx)?;
        let byte = idx / 8;
        let bit = idx % 8;
        Ok((self.bytes[byte] & (1 << bit)) != 0)
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.iter().all(|&b| b == 0)
    }

    pub fn clear_all(&mut self) {
        self.bytes.fill(0);
    }
}

//---
// ## Hexadecimal Utilities
// Functions for writing bytes in hexadecimal format to a stream.

fn nibble_to_hex(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'A' + n - 10) as char,
        _ => '?',
    }
}

pub fn write_byte_hex_fmt<W: core::fmt::Write>(w: &mut W, byte: u8) -> core::fmt::Result {
    let hi = byte >> 4;
    let lo = byte & 0x0F;
    w.write_char(nibble_to_hex(hi))?;
    w.write_char(nibble_to_hex(lo))?;
    Ok(())
}

pub fn write_bytes_hex_fmt<W: core::fmt::Write>(w: &mut W, bytes: &[u8]) -> core::fmt::Result {
    for (i, &b) in bytes.iter().enumerate() {
        write_byte_hex_fmt(w, b)?;
        if i != bytes.len() - 1 {
            w.write_char(' ')?;
        }
    }
    Ok(())
}

pub fn write_bytes_hex_prefixed_fmt<W: core::fmt::Write>(
    w: &mut W,
    bytes: &[u8],
) -> core::fmt::Result {
    for (i, &b) in bytes.iter().enumerate() {
        w.write_str("0x")?;
        write_byte_hex_fmt(w, b)?;
        if i != bytes.len() - 1 {
            w.write_char(' ')?;
        }
    }
    Ok(())
}

//---
// ## String and Character Utilities
// Functions for writing strings and handling character encodings.

/// Writes a string byte by byte to a writer.
pub fn write_str_byte<W: embedded_io::Write>(writer: &mut W, s: &str) -> Result<(), W::Error> {
    writer.write_all(s.as_bytes())?;
    Ok(())
}

/// A wrapper that ensures all output is ASCII-safe by escaping non-ASCII characters.
struct AsciiSafeWriter<'a, W: 'a + core::fmt::Write>(&'a mut W);

impl<'a, W: core::fmt::Write> core::fmt::Write for AsciiSafeWriter<'a, W> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut last = 0;
        for (idx, c) in s.char_indices() {
            if !c.is_ascii() {
                if last < idx {
                    self.0.write_str(&s[last..idx])?;
                }
                write!(self.0, "\\u{{{:X}}}", c as u32)?;
                last = idx + c.len_utf8();
            }
        }
        if last < s.len() {
            self.0.write_str(&s[last..])?;
        }
        Ok(())
    }
}

/// Writes a formatted string to a writer, ensuring all characters are ASCII-safe.
///
/// This function is the robust, no-alloc replacement for `prevent_garbled` and
/// `write_ascii_safe`, handling formatting and escaping in a single pass.
pub fn write_formatted_ascii_safe<S: core::fmt::Write>(
    serial: &mut S,
    args: core::fmt::Arguments<'_>,
) -> Result<(), core::fmt::Error> {
    let mut writer = AsciiSafeWriter(serial);
    core::fmt::Write::write_fmt(&mut writer, args)
}

//---
// ## Deprecated Utilities
// These functions are similar to the new ones and have been made redundant.

pub const fn calculate_cmd_buffer_size(num_commands: usize, max_cmd_len: usize) -> usize {
    num_commands * (max_cmd_len + 1) + num_commands * 2
}
