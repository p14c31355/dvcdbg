//! I2C command sequence explorer for no_std with set-unit dependency check
use heapless::Vec;
use crate::scanner::{I2C_SCAN_ADDR_START, I2C_SCAN_ADDR_END};

pub struct CmdNode<'a> {
    pub cmd: u8,
    pub deps: &'a [u8],
    pub sets: &'a [&'a [u8]],
}

pub struct Explorer<'a> {
    pub sequence: &'a [CmdNode<'a>],
}

impl<'a> Explorer<'a> {
    pub fn explore<I2C, W>(
        &self,
        i2c: &mut I2C,
        serial: &mut W,
    ) where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
    {
        let mut current_seq: Vec<u8, 32> = Vec::new();
        self.backtrack(i2c, serial, 0, &mut current_seq);
    }

    fn backtrack<I2C, W>(
        &self,
        i2c: &mut I2C,
        serial: &mut W,
        index: usize,
        current_seq: &mut Vec<u8, 32>,
    ) where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
    {
        if index >= self.sequence.len() {
            let _ = writeln!(serial, "[explorer] Valid sequence found: {:?}", current_seq);
            return;
        }

        let node = &self.sequence[index];

        let deps_ok = node.deps.iter().all(|d| current_seq.contains(d));

        let sets_ok = node.sets.is_empty() || node.sets.iter().any(|set| set.iter().all(|s| current_seq.contains(s)));

        if deps_ok && sets_ok {
            if current_seq.push(node.cmd).is_ok() {
                for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                    let _ = i2c.write(addr, &[node.cmd]);
                }
                self.backtrack(i2c, serial, index + 1, current_seq);
                let _ = current_seq.pop();
            }
        }

        self.backtrack(i2c, serial, index + 1, current_seq);
    }
}
