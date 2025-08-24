use crate::scanner::{I2C_SCAN_ADDR_END, I2C_SCAN_ADDR_START};
use heapless::Vec;

const CMD_CAPACITY: usize = 32;
const MAX_PERMUTATION_WARNING_THRESHOLD: usize = 8;

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
        if !remaining.is_empty() {
            let _ = writeln!(
                serial,
                "[explorer] warning: unresolved dependencies found, possibly due to a cycle."
            );
        }

        let _ = writeln!(serial, "[explorer] staged: {staged:?}");
        let _ = writeln!(serial, "[explorer] unresolved: {remaining:?}");

        // Now, unresolved must be permuted
        let mut current: Vec<u8, CMD_CAPACITY> = staged.clone();
        let mut used = [false; CMD_CAPACITY];
        if remaining.len() > MAX_PERMUTATION_WARNING_THRESHOLD {
            let _ = writeln!(
                serial,
                "[explorer] warning: Large number of unresolved commands ({}). This may take a very long time.",
                remaining.len()
            );
        }
        let mut current_set = staged_set;
        self.permute(
            i2c,
            serial,
            &remaining,
            &mut current,
            &mut used,
            &mut current_set,
        );
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
    ) where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
    {
        // Stack to store the 'pos' (index in unresolved.iter().enumerate()) of the element
        // that was added to 'current' at the current depth. This allows us to backtrack.
        let mut path_stack: Vec<usize, CMD_CAPACITY> = Vec::new();

        // Stack to store the starting index for the 'for' loop at each depth.
        // When we go deeper, we push 0. When we backtrack, we increment the top.
        let mut loop_start_indices: Vec<usize, CMD_CAPACITY> = Vec::new();
        loop_start_indices.push(0); // Initial depth starts loop from index 0

        'main_loop: loop {
            if current.len() == self.sequence.len() {
                // Base case: A full permutation has been formed.
                let _ = writeln!(serial, "[explorer] candidate: {current:?}");

                for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                    let all_ok = current.iter().all(|&cmd| i2c.write(addr, &[cmd]).is_ok());
                    if all_ok {
                        let _ = writeln!(
                            serial,
                            "[explorer] success: sequence {current:?} works for addr 0x{addr:02X}"
                        );
                    }
                }

                if !self.backtrack(
                    unresolved,
                    current,
                    used,
                    current_set,
                    &mut path_stack,
                    &mut loop_start_indices,
                    false,
                ) {
                    break 'main_loop;
                }
            } else {
                // Recursive step: Try to extend the current permutation.
                let mut found_next_candidate = false;
                let current_loop_start_idx = *loop_start_indices.last().unwrap();

                for (pos, &idx) in unresolved.iter().enumerate().skip(current_loop_start_idx) {
                    if used[pos] {
                        continue;
                    }
                    let node = &self.sequence[idx];
                    if node.deps.iter().all(|d| current_set[*d as usize]) {
                        // Make the choice: add this command.
                        current.push(node.cmd).unwrap();
                        current_set[node.cmd as usize] = true;
                        used[pos] = true;

                        path_stack.push(pos);
                        loop_start_indices.push(0);
                        found_next_candidate = true;
                        break;
                    }
                }

                if !found_next_candidate {
                    // No more candidates at the current depth, backtrack.
                    if !self.backtrack(
                        unresolved,
                        current,
                        used,
                        current_set,
                        &mut path_stack,
                        &mut loop_start_indices,
                        true,
                    ) {
                        break 'main_loop;
                    }
                }
            }
        }
    }

    fn backtrack(
        &self,
        unresolved: &Vec<usize, CMD_CAPACITY>,
        current: &mut Vec<u8, CMD_CAPACITY>,
        used: &mut [bool; CMD_CAPACITY],
        current_set: &mut [bool; 256],
        path_stack: &mut Vec<usize, CMD_CAPACITY>,
        loop_start_indices: &mut Vec<usize, CMD_CAPACITY>,
        pop_loop_index: bool,
    ) -> bool {
        if let Some(last_added_pos) = path_stack.pop() {
            let node_cmd = self.sequence[unresolved[last_added_pos]].cmd;
            used[last_added_pos] = false;
            current_set[node_cmd as usize] = false;
            current.pop();
            if pop_loop_index {
                loop_start_indices.pop();
            }
            if let Some(last_loop_idx) = loop_start_indices.last_mut() {
                *last_loop_idx += 1;
            } else {
                // path_stack is empty, all permutations explored.
                return false;
            }
            true
        } else {
            // path_stack is empty, all permutations explored.
            false
        }
    }
}
