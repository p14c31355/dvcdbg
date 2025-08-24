//! Stage-based I2C command sequence explorer for no_std
use heapless::Vec;
use crate::scanner::{I2C_SCAN_ADDR_START, I2C_SCAN_ADDR_END};

pub struct CmdNode<'a> {
    pub cmd: u8,
    pub deps: &'a [u8],
    pub stage: usize,
}

pub struct Explorer<'a> {
    pub sequence: &'a [CmdNode<'a>],
    pub max_stage: usize,
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
        self.backtrack(i2c, serial, 0, 0, &mut current_seq);
    }

    fn backtrack<I2C, W>(
        &self,
        i2c: &mut I2C,
        serial: &mut W,
        index: usize,
        stage: usize,
        current_seq: &mut Vec<u8, 32>,
    ) where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
    {
        if index >= self.sequence.len() || stage >= self.max_stage {
            let _ = writeln!(serial, "[explorer] Stage {} sequence: {:?}", stage, current_seq);
            return;
        }

        let node = &self.sequence[index];

        if node.deps.iter().all(|d| current_seq.contains(d)) && node.stage <= stage {
            for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                let _ = i2c.write(addr, &[node.cmd]);
            }

            if current_seq.push(node.cmd).is_ok() {
                let next_stage = if current_seq.len() % (self.max_stage / 4).max(1) == 0 {
                    stage + 1
                } else {
                    stage
                };
                self.backtrack(i2c, serial, index + 1, next_stage, current_seq);
                let _ = current_seq.pop();
            }
        }

        self.backtrack(i2c, serial, index + 1, stage, current_seq);
    }
}
