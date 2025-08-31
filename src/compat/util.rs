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
use crate::error::BitFlagsError;

pub struct BitFlags<const N: usize, const S: usize> {
    bytes: [u8; S],
}

impl<const N: usize, const S: usize> BitFlags<N, S> {
    pub fn new() -> Self {
        Self {
            vec: heapless::Vec::from_slice(&[0u8; S]).unwrap(),
        }
    }

    pub fn set(&mut self, idx: usize) -> Result<(), BitFlagsError> {
        if idx >= N {
            return Err(BitFlagsError::IndexOutOfBounds { idx, max: N - 1 });
        }
        let byte = idx / 8;
        let bit = idx % 8;
        self.vec[byte] |= 1 << bit;
        Ok(())
    }

    pub fn clear(&mut self, idx: usize) -> Result<(), BitFlagsError> {
        if idx >= N {
            return Err(BitFlagsError::IndexOutOfBounds { idx, max: N - 1 });
        }
        let byte = idx / 8;
        let bit = idx % 8;
        self.vec[byte] &= !(1 << bit);
        Ok(())
    }

    pub fn get(&self, idx: usize) -> Result<bool, BitFlagsError> {
        if idx >= N {
            return Err(BitFlagsError::IndexOutOfBounds { idx, max: N - 1 });
        }
        let byte = idx / 8;
        let bit = idx % 8;
        Ok((self.vec[byte] & (1 << bit)) != 0)
    }
}

const UART_CHUNK_SIZE: usize = 64;

pub fn prevent_garbled<W: core::fmt::Write>(serial: &mut W, args: core::fmt::Arguments) {
    let mut buffer = heapless::String::<512>::new();
    core::fmt::Write::write_fmt(&mut buffer, args).ok();

    let mut start = 0;
    while start < buffer.len() {
        let end = (start + UART_CHUNK_SIZE).min(buffer.len());
        writeln!(serial, "{}", &buffer[start..end]).ok();
        start = end;
    }
}
