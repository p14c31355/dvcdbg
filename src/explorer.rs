use heapless::Vec;
use crate::scanner::{I2C_SCAN_ADDR_START, I2C_SCAN_ADDR_END};

const CMD_CAPACITY: usize = 32;

pub struct CmdNode<'a> {
    pub cmd: u8,
    pub deps: &'a [u8],
}

pub struct Explorer<'a> {
    pub sequence: &'a [CmdNode<'a>],
}

impl<'a> Explorer<'a> {
    pub fn explore<I2C, W>(&self, i2c: &mut I2C, serial: &mut W) -> Result<(), ()>
    where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
    {
        // iterative staging (topological sort-like)
        let mut staged: Vec<u8, CMD_CAPACITY> = Vec::new();
        let mut remaining: Vec<usize, CMD_CAPACITY> = Vec::new();
        for i in 0..self.sequence.len() {
            if remaining.push(i).is_err() {
                let _ = writeln!(serial, "error: too many commands");
                return Err(());
            }
        }

        loop {
            let before = staged.len();
            let mut new_remaining: Vec<usize, CMD_CAPACITY> = Vec::new();

            for &idx in remaining.iter() {
                let node = &self.sequence[idx];
                if node.deps.iter().all(|d| staged.contains(d)) {
                    if staged.push(node.cmd).is_err() {
                        let _ = writeln!(serial, "error: staged buffer full");
                        return Err(());
                    }
                } else {
                    let _ = new_remaining.push(idx);
                }
            }

            remaining = new_remaining;
            if staged.len() == before {
                // No progress was made in this iteration, so we break.
                break;
            }
        }

        let _ = writeln!(serial, "[explorer] staged: {:?}", staged);
        let _ = writeln!(serial, "[explorer] unresolved: {:?}", remaining);

        // Now, unresolved must be permuted
        let mut current: Vec<u8, CMD_CAPACITY> = staged.clone();
        let mut used = [false; CMD_CAPACITY];
        self.permute(i2c, serial, &remaining, &mut current, &mut used)?;

        Ok(())
    }

    fn permute<I2C, W>(
        &self,
        i2c: &mut I2C,
        serial: &mut W,
        unresolved: &Vec<usize, CMD_CAPACITY>,
        current: &mut Vec<u8, CMD_CAPACITY>,
        used: &mut [bool; CMD_CAPACITY],
    ) -> Result<(), ()>
    where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
    {
        if current.len() == self.sequence.len() {
            let _ = writeln!(serial, "[explorer] candidate: {:?}", current);

            for &cmd in current.iter() {
                for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                    if let Err(e) = i2c.write(addr, &[cmd]) {
                      let _ = writeln!(serial, "i2c error at addr 0x{:02X}: {:?}", addr, e);
                    }
                }
            }
            return Ok(());
        }

        for (pos, &idx) in unresolved.iter().enumerate() {
            if used[pos] {
                continue;
            }
            let node = &self.sequence[idx];
            if node.deps.iter().all(|d| current.contains(d)) {
                if current.push(node.cmd).is_err() {
                    return Err(());
                }
                used[pos] = true;
                self.permute(i2c, serial, unresolved, current, used)?;
                used[pos] = false;
                current.pop();
            }
        }
        Ok(())
    }
}
