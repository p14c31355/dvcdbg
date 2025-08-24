use heapless::Vec;
use crate::scanner::{I2C_SCAN_ADDR_START, I2C_SCAN_ADDR_END};

pub struct CmdNode<'a> {
    pub cmd: u8,
    pub deps: &'a [u8],
}

pub struct Explorer<'a> {
    pub sequence: &'a [CmdNode<'a>],
}

impl<'a> Explorer<'a> {
    pub fn explore<I2C, W>(&self, i2c: &mut I2C, serial: &mut W)
    where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
    {
        let mut staged_seq: Vec<u8, 32> = Vec::new();
        let mut problem_indices: Vec<usize, 16> = Vec::new();

        for (i, node) in self.sequence.iter().enumerate() {
            if node.deps.iter().all(|d| staged_seq.contains(d)) {
                for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                    let _ = i2c.write(addr, &[node.cmd]);
                }
                let _ = staged_seq.push(node.cmd);
            } else {
                let _ = problem_indices.push(i);
            }
        }

        let _ = writeln!(serial, "[explorer] Staged sequence: {:?}", staged_seq);
        let _ = writeln!(serial, "[explorer] Problem indices: {:?}", problem_indices);

        let mut combo: Vec<u8, 16> = Vec::new();
        self.backtrack_combo(i2c, serial, &problem_indices, 0, &mut combo, &staged_seq);
    }

    fn backtrack_combo<I2C, W>(
        &self,
        i2c: &mut I2C,
        serial: &mut W,
        problem_indices: &[usize],
        depth: usize,
        combo: &mut Vec<u8, 16>,
        staged_seq: &Vec<u8, 32>,
    ) where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
    {
        if depth >= problem_indices.len() {
            if self.check_deps(combo, staged_seq) {
                let mut full_seq = staged_seq.clone();
                full_seq.extend(combo.iter().copied());
                let _ = writeln!(serial, "[explorer] Valid full sequence: {:?}", full_seq);
                for &cmd in full_seq.iter() {
                    for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                        let _ = i2c.write(addr, &[cmd]);
                    }
                }
            }
            return;
        }

        let idx = problem_indices[depth];
        let node = &self.sequence[idx];

        if combo.push(node.cmd).is_ok() {
            self.backtrack_combo(i2c, serial, problem_indices, depth + 1, combo, staged_seq);
            let _ = combo.pop();
        }

        self.backtrack_combo(i2c, serial, problem_indices, depth + 1, combo, staged_seq);
    }

    fn check_deps(&self, combo: &Vec<u8, 16>, staged_seq: &Vec<u8, 32>) -> bool {
        let mut accumulated = staged_seq.clone();
        accumulated.extend(combo.iter().copied());
        for node in self.sequence.iter() {
            if !node.deps.iter().all(|d| accumulated.contains(d)) {
                return false;
            }
        }
        true
    }
}
