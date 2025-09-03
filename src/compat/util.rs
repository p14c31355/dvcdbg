//! src/compat/util.rs
pub const fn calculate_cmd_buffer_size(num_commands: usize, max_cmd_len: usize) -> usize {
    num_commands * (max_cmd_len + 1) + num_commands * 2
}

pub const ERROR_STRING_BUFFER_SIZE: usize = 128;

/// AVR / no_std support ASCII utility
use embedded_io::Write;

pub fn write_byte_hex<W: Write>(w: &mut W, byte: u8) -> Result<(), W::Error> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let hi = HEX[(byte >> 4) as usize];
    let lo = HEX[(byte & 0x0F) as usize];
    w.write(&[hi])?;
    w.write(&[lo])?;
    Ok(())
}

pub fn write_bytes_hex<W: Write>(w: &mut W, bytes: &[u8]) -> Result<(), W::Error> {
    let mut it = bytes.iter().peekable();
    while let Some(&b) = it.next() {
        write_byte_hex(w, b)?;
        if it.peek().is_some() {
            w.write(b" ")?;
        }
    }
    Ok(())
}

pub fn write_bytes_hex_prefixed<W: Write>(w: &mut W, bytes: &[u8]) -> Result<(), W::Error> {
    let mut it = bytes.iter().peekable();
    while let Some(&b) = it.next() {
        w.write(b"0x")?;
        write_byte_hex(w, b)?;
        if it.peek().is_some() {
            w.write(b" ")?;
        }
    }
    Ok(())
}

pub fn write_bytes_hex_line<W: Write>(w: &mut W, bytes: &[u8]) -> Result<(), W::Error> {
    write_bytes_hex_prefixed(w, bytes)?;
    w.write(b"\r\n")?;
    Ok(())
}

pub fn write_bytes_hex_fmt<W: core::fmt::Write>(w: &mut W, bytes: &[u8]) -> core::fmt::Result {
    for (i, &b) in bytes.iter().enumerate() {
        let hi = b >> 4;
        let lo = b & 0x0F;
        w.write_char(nibble_to_hex(hi))?;
        w.write_char(nibble_to_hex(lo))?;
        if i != bytes.len() - 1 {
            w.write_char(' ')?;
        }
    }
    Ok(())
}

fn nibble_to_hex(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'A' + n - 10) as char,
        _ => '?',
    }
}

// bitmask utility
// src/compat/util.rs
use crate::error::BitFlagsError;

// For stable Rust, const generic expressions like `(N + 7) / 8` are not allowed
// in array lengths within struct definitions. This feature (`generic_const_exprs`)
// is currently unstable.

/// A bitflag structure optimized for 128 bits, used for tracking I2C addresses.
pub struct BitFlags {
    bytes: [u8; 16],
}
//
// Since `BitFlags` is primarily used with `I2C_ADDRESS_COUNT` (128 bits),
// we can make it concrete for this specific size to ensure compilation on stable Rust.
// (128 bits requires (128 + 7) / 8 = 16 bytes).
impl Default for BitFlags {
    fn default() -> Self {
        Self::new()
    }
}

impl BitFlags {
    pub const fn new() -> Self {
        Self { bytes: [0u8; 16] }
    }

    // N_BITS is now implicitly 128 for this concrete implementation
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

const UART_CHUNK_SIZE: usize = 256;

pub fn prevent_garbled<W: core::fmt::Write>(serial: &mut W, args: core::fmt::Arguments) {
    let mut buffer = heapless::String::<512>::new();
    core::fmt::Write::write_fmt(&mut buffer, args).ok();

    let mut start = 0;
    while start < buffer.len() {
        let mut end = (start + UART_CHUNK_SIZE).min(buffer.len());
        if end < buffer.len() {
            while end > start && !buffer.is_char_boundary(end) {
                end -= 1;
            }
        }

        // If no boundary was found, end could be equal to start.
        // To prevent an infinite loop, if end == start, we must send something.
        if end == start {
            end = (start + UART_CHUNK_SIZE).min(buffer.len());
        }

        if start < end {
            writeln!(serial, "{}", &buffer[start..end]).ok();
        }
        start = end;
    }
}

pub fn write_str_bytewise<W: core::fmt::Write>(serial: &mut W, s: &str) {
    for b in s.as_bytes() {
        let _ = serial.write_char(*b as char);
    }
}

pub fn write_str_byte<W: Write>(writer: &mut W, s: &str) -> Result<(), W::Error> {
    for &byte in s.as_bytes() {
        writer.write_all(&[byte])?;
    }
    Ok(())
}

pub fn write_ascii_safe<S: core::fmt::Write>(
    serial: &mut S,
    text: &str
) -> Result<(), core::fmt::Error> {
    for c in text.chars() {
        if c.is_ascii() {
            write!(serial, "{}", c)?;
        } else {
            write!(serial, "\\u{{{:X}}}", c as u32)?;
        }
    }
    Ok(())
}

pub fn write_formatted_ascii_safe<S: core::fmt::Write>(
    serial: &mut S,
    args: core::fmt::Arguments<'_>,
) -> Result<(), core::fmt::Error> {
    struct AsciiSafeWriter<'a, W: 'a + core::fmt::Write>(&'a mut W);

    impl<'a, W: core::fmt::Write> core::fmt::Write for AsciiSafeWriter<'a, W> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for c in s.chars() {
                if c.is_ascii() {
                    write!(self.0, "{}", c)?;
                } else {
                    write!(self.0, "\\u{{{:X}}}", c as u32)?;
                }
            }
            Ok(())
        }
    }

    let mut writer = AsciiSafeWriter(serial);
    core::fmt::Write::write_fmt(&mut writer, args)
}

pub fn write_node_deps<W: core::fmt::Write>(
    w: &mut W,
    index: usize,
    deps: &[u8],
) -> core::fmt::Result {
    if index < 256 {
        write!(w, "node {:02X}: deps=", index as u8)?;
    } else {
        write!(w, "node ?: deps=")?;
    }
    write_bytes_hex_fmt(w, deps)?;
    writeln!(w)
}
