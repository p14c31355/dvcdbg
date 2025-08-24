use crate::scanner::{I2C_SCAN_ADDR_END, I2C_SCAN_ADDR_START};
use heapless::Vec;

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
        if self.sequence.len() > CMD_CAPACITY {
            let _ = writeln!(serial, "error: too many commands");
            return Err(());
        }
        let mut remaining: Vec<usize, CMD_CAPACITY> = (0..self.sequence.len()).collect();
        let mut staged_set = [false; 256];

        loop {
            let before = staged.len();
            remaining.retain(|&idx| {
                let node = &self.sequence[idx];
                if node.deps.iter().all(|d| staged_set[*d as usize]) {
                    // This can't fail due to the check above.
                    staged.push(node.cmd).unwrap();
                    staged_set[node.cmd as usize] = true;
                    false // remove from remaining
                } else {
                    true // keep in remaining
                }
            });

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
        let mut current_set = [false; 256];
        for &cmd in current.iter() {
            current_set[cmd as usize] = true;
        }
        self.permute(
            i2c,
            serial,
            &remaining,
            &mut current,
            &mut used,
            &mut current_set,
        )?;
        Ok(())
    }

    fn permute<I2C, W>(
        &self,
        i2c: &mut I2C,
        serial: &mut W,
        unresolved: &Vec<usize, CMD_CAPACITY>,
        current: &mut Vec<u8, CMD_CAPACITY>,
        used: &mut [bool; CMD_CAPACITY],
        current_set: &mut [bool; 256],
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
            if node.deps.iter().all(|d| current_set[*d as usize]) {
                // This can't fail if the initial length check is done in `explore`.
                current.push(node.cmd).unwrap();
                current_set[node.cmd as usize] = true;
                used[pos] = true;
                self.permute(i2c, serial, unresolved, current, used, current_set)?;
                used[pos] = false;
                current_set[node.cmd as usize] = false;
                current.pop();
            }
        }
        Ok(())
    }
}
