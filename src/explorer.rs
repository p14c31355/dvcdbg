use crate::scanner::{I2C_SCAN_ADDR_END, I2C_SCAN_ADDR_START};
use heapless::Vec;

const CMD_CAPACITY: usize = 32;
const MAX_PERMUTATION_WARNING_THRESHOLD: usize = 8;
const I2C_ADDRESS_COUNT: usize = 128;

pub enum ExplorerError {
    TooManyCommands,
}

pub struct CmdNode<'a> {
    pub cmd: u8,
    pub deps: &'a [u8],
}

pub struct Explorer<'a> {
    pub sequence: &'a [CmdNode<'a>],
}

struct PermutationState<const C: usize> {
    current: Vec<u8, C>,
    used: [bool; C],
    current_set: [bool; 256],
    path_stack: Vec<usize, C>,
    loop_start_indices: Vec<usize, C>,
}

impl<'a> Explorer<'a> {
    pub fn explore<I2C, W>(&self, i2c: &mut I2C, serial: &mut W) -> Result<(), ExplorerError>
    where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
    {
        let mut staged: Vec<u8, CMD_CAPACITY> = Vec::new();
        if self.sequence.len() > CMD_CAPACITY {
            let _ = writeln!(serial, "error: too many commands");
            return Err(ExplorerError::TooManyCommands);
        }

        let mut remaining: Vec<usize, CMD_CAPACITY> = (0..self.sequence.len()).collect();
        let mut staged_set = [false; 256];

        loop {
            let before = staged.len();
            remaining.retain(|&idx| {
                let node = &self.sequence[idx];
                if node.deps.iter().all(|d| staged_set[*d as usize]) {
                    staged
                        .push(node.cmd)
                        .expect("staged vec should have enough capacity");
                    staged_set[node.cmd as usize] = true;
                    false
                } else {
                    true
                }
            });
            if staged.len() == before {
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

        let mut current_state = PermutationState {
            current: staged.clone(),
            used: [false; CMD_CAPACITY],
            current_set: staged_set,
            path_stack: Vec::new(),
            loop_start_indices: Vec::from_slice(&[0]).unwrap(),
        };

        let mut solved_addrs = [false; I2C_ADDRESS_COUNT];

        if remaining.len() > MAX_PERMUTATION_WARNING_THRESHOLD {
            let _ = writeln!(
                serial,
                "[explorer] warning: Large number of unresolved commands ({}). This may take a very long time.",
                remaining.len()
            );
        }

        self.permute(
            i2c,
            serial,
            &remaining,
            &mut current_state,
            &mut solved_addrs,
        );

        Ok(())
    }

    fn permute<I2C, W>(
        &self,
        i2c: &mut I2C,
        serial: &mut W,
        unresolved: &Vec<usize, CMD_CAPACITY>,
        state: &mut PermutationState<CMD_CAPACITY>,
        solved_addrs: &mut [bool; 128],
    ) where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
    {
        'main_loop: loop {
            if state.current.len() == self.sequence.len() {
                self.handle_full_permutation(i2c, serial, state, solved_addrs);
                if !self.backtrack(unresolved, state, false) {
                    break 'main_loop;
                }
            } else if !self.try_extend_permutation(unresolved, state) {
                // Could not extend, backtrack
                if !self.backtrack(unresolved, state, true) {
                    break 'main_loop;
                }
            }
        }
    }

    fn handle_full_permutation<I2C, W>(
        &self,
        i2c: &mut I2C,
        serial: &mut W,
        state: &mut PermutationState<CMD_CAPACITY>,
        solved_addrs: &mut [bool; 128],
    ) where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
    {
        let _ = writeln!(serial, "[explorer] candidate: {:?}", state.current);

        for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
            if solved_addrs[addr as usize] {
                continue;
            }
            let all_ok = state
                .current
                .iter()
                .all(|&cmd| i2c.write(addr, &[cmd]).is_ok());
            if all_ok {
                let _ = writeln!(
                    serial,
                    "[explorer] success: sequence {:?} works for addr 0x{:02X}",
                    state.current, addr
                );
                solved_addrs[addr as usize] = true;
            }
        }
    }

    fn try_extend_permutation(
        &self,
        unresolved: &Vec<usize, CMD_CAPACITY>,
        state: &mut PermutationState<CMD_CAPACITY>,
    ) -> bool {
        let current_loop_start_idx = *state
            .loop_start_indices
            .last()
            .expect("loop_start_indices should not be empty");

        for (pos, &idx) in unresolved.iter().enumerate().skip(current_loop_start_idx) {
            if state.used[pos] {
                continue;
            }
            let node = &self.sequence[idx];
            if node.deps.iter().all(|d| state.current_set[*d as usize]) {
                // Make choice
                state.current.push(node.cmd).unwrap();
                state.current_set[node.cmd as usize] = true;
                state.used[pos] = true;

                let _ = state.path_stack.push(pos);
                let _ = state.loop_start_indices.push(0);
                return true;
            }
        }
        false
    }

    fn backtrack(
        &self,
        unresolved: &Vec<usize, CMD_CAPACITY>,
        state: &mut PermutationState<CMD_CAPACITY>,
        pop_loop_index: bool,
    ) -> bool {
        if let Some(last_added_pos) = state.path_stack.pop() {
            let node_cmd = self.sequence[unresolved[last_added_pos]].cmd;
            state.used[last_added_pos] = false;
            state.current_set[node_cmd as usize] = false;
            state.current.pop();
            if pop_loop_index {
                state.loop_start_indices.pop();
            }
            if let Some(last_loop_idx) = state.loop_start_indices.last_mut() {
                *last_loop_idx += 1;
            } else {
                return false;
            }
            true
        } else {
            false
        }
    }
}
