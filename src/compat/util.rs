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
        write!(w, "{b:02X}")?;
        if i != bytes.len() - 1 {
            write!(w, " ")?;
        }
    }
    Ok(())
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
    pub fn new() -> Self {
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
}

const UART_CHUNK_SIZE: usize = 64;

pub fn prevent_garbled<W: core::fmt::Write>(serial: &mut W, args: core::fmt::Arguments) {
    let mut buffer = heapless::String::<256>::new();
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

pub fn write_node_deps<W: core::fmt::Write>(
    w: &mut W,
    index: usize,
    deps: &[u8],
) -> core::fmt::Result {
    write!(w, "node ")?;
    if index < 256 {
        write!(w, "{:02X}", index as u8)?;
    } else {
        write!(w, "?")?; // overflow fallback
    }
    write!(w, ": deps=")?;
    write_bytes_hex_fmt(w, deps)?;
    writeln!(w)?; // Add a newline since write_bytes_hex_fmt doesn't include one
    Ok(())
}
