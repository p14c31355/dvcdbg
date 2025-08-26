//! # I2C Command Sequence Explorer (Refactored)
//!
//! This module provides an algorithm to discover valid sequences of I2C commands
//! for devices with dependency constraints, with a focus on embedded systems.
//!
//! ## Overview
//! - The `Explorer` is now a dependency graph manager that generates valid command permutations.
//! - `PermutationIter` is a stack-safe, non-recursive iterator for these permutations.
//! - The `CmdExecutor` and `Logger` traits allow for customization of I2C operations and logging.
//!
//! ## Key Refinements
//! 1. **Separation of Concerns**: The core algorithm (`PermutationIter`) is separate from
//!    the I2C execution logic (`explore`).
//! 2. **Iterator-based API**: The `permutations()` method returns an iterator,
//!    making the code more testable and composable.
//! 3. **Generic Capacity**: `CMD_CAPACITY` is now a generic parameter `N`,
//!    allowing for code reuse across devices with different memory constraints.
//! 4. **Abstracted Logging**: A `Logger` trait is introduced for flexible logging,
//!    reducing RAM/ROM usage on tiny microcontrollers.
//! 5. **Robust Error Handling**: The `ExplorerError` enum is expanded to include
//!    dependency cycles and other runtime issues.
//!
//! ## Usage
//! ```ignore
//! use crate::{Explorer, CmdNode, I2cCompat, Logger, CmdExecutor};
//!
//! // Example implementations for a specific platform
//! struct MyExecutor;
//! impl<I2C: I2cCompat> CmdExecutor<I2C> for MyExecutor { /* ... */ }
//! struct NullLogger;
//! impl Logger for NullLogger { /* ... */ }
//!
//! // Define commands with dependencies
//! let cmds = &[
//!     CmdNode { bytes: &[0x01], deps: &[] },
//!     CmdNode { bytes: &[0x02], deps: &[0x01] },
//!     CmdNode { bytes: &[0x03], deps: &[0x01] },
//! ];
//!
//! // The generic parameter 32 matches the capacity of the Vecs
//! let explorer = Explorer::<32> { sequence: cmds };
//! let mut executor = MyExecutor;
//! let mut logger = NullLogger;
//! let mut i2c = // ...
//!
//! let result = explorer.explore(&mut i2c, &mut executor, &mut logger);
//! if let Err(e) = result {
//!     logger.log_error(&format!("Exploration failed: {:?}", e));
//! }
//! ```

use core::fmt::Write;
use heapless::{String, Vec};

const I2C_SCAN_ADDR_END: u8 = 127;
const I2C_SCAN_ADDR_START: u8 = 1;
const I2C_ADDRESS_COUNT: usize = 128;
const LOG_BUFFER_CAPACITY: usize = 512;

/// Errors that can occur during exploration of command sequences.
#[derive(Debug, PartialEq, Eq)]
pub enum ExplorerError {
    /// The provided sequence contained more commands than supported by the capacity N.
    TooManyCommands,
    /// The command dependency graph contains a cycle.
    DependencyCycle,
    /// No valid I2C addresses were found for any command sequence.
    NoValidAddressesFound,
    /// An I2C command execution failed.
    ExecutionFailed,
}

/// Represents a single I2C command in the dependency graph.
///
/// The dependency is now on the index of the dependent command in the sequence.
#[derive(Copy, Clone)]
pub struct CmdNode<'a, const N: usize> {
    /// The I2C command bytes to be sent.
    pub bytes: &'a [u8],
    /// The indices of the commands that must precede this command.
    pub deps: &'a [usize],
}

/// A trait for executing a command on an I2C bus.
pub trait CmdExecutor<I2C> {
    /// Executes a given command byte sequence.
    fn exec(&mut self, i2c: &mut I2C, addr: u8, cmd: &[u8]) -> Result<(), ()>;
}

/// A trait for logging progress and results.
pub trait Logger {
    fn log_info(&mut self, msg: &str);
    fn log_warning(&mut self, msg: &str);
    fn log_error(&mut self, msg: &str);
}

// Dummy logger for platforms without console output
pub struct NullLogger;
impl Logger for NullLogger {
    fn log_info(&mut self, _msg: &str) {}
    fn log_warning(&mut self, _msg: &str) {}
    fn log_error(&mut self, _msg: &str) {}
}

/// The core explorer, now a generic dependency graph manager.
pub struct Explorer<'a, const N: usize> {
    pub sequence: &'a [CmdNode<'a, N>],
}

/// An iterator that generates valid I2C command permutations.
pub struct PermutationIter<'a, const N: usize> {
    staged: Vec<&'a [u8], N>,
    unresolved_indices: Vec<usize, N>,
    current: Vec<&'a [u8], N>,
    used: [bool; N],
    staged_and_current_set: [bool; 256],
    path_stack: Vec<usize, N>,
    loop_start_indices: Vec<usize, N>,
    is_done: bool,
}

impl<'a, const N: usize> Explorer<'a, N> {
    /// Performs an initial topological sort to stage commands without unresolved dependencies.
    fn stage(&self) -> Result<(Vec<&'a [u8], N>, Vec<usize, N>), ExplorerError> {
        if self.sequence.len() > N {
            return Err(ExplorerError::TooManyCommands);
        }
        let mut staged: Vec<&'a [u8], N> = Vec::new();
        let mut remaining: Vec<usize, N> = (0..self.sequence.len()).collect();
        let mut staged_set = [false; 256];

        loop {
            let before = staged.len();
            let mut newly_staged_count = 0;
            let mut i = 0;
            while i < remaining.len() {
                let node_idx = remaining[i];
                let node = &self.sequence[node_idx];
                let deps_satisfied = node.deps.iter().all(|&d| {
                    if d >= self.sequence.len() {
                        return false; // Invalid dependency index
                    }
                    if let Some(first_byte) = self.sequence[d].bytes.first() {
                        staged_set[*first_byte as usize]
                    } else {
                        false
                    }
                });

                if deps_satisfied {
                    staged
                        .push(node.bytes)
                        .expect("staged vec should have enough capacity");
                    if let Some(first_byte) = node.bytes.first() {
                        staged_set[*first_byte as usize] = true;
                    }
                    remaining.swap_remove(i);
                    newly_staged_count += 1;
                } else {
                    i += 1;
                }
            }

            if staged.len() == before {
                if !remaining.is_empty() {
                    return Err(ExplorerError::DependencyCycle);
                }
                break;
            }
        }
        Ok((staged, remaining))
    }

    /// Returns a stack-safe iterator for all valid command permutations.
    pub fn permutations(&self) -> Result<PermutationIter<'a, N>, ExplorerError> {
        let (staged, unresolved_indices) = self.stage()?;

        let mut staged_and_current_set = [false; 256];
        for cmd_bytes in staged.iter() {
            if let Some(first_byte) = cmd_bytes.first() {
                staged_and_current_set[*first_byte as usize] = true;
            }
        }

        Ok(PermutationIter {
            staged,
            unresolved_indices,
            current: Vec::new(),
            used: [false; N],
            staged_and_current_set,
            path_stack: Vec::new(),
            loop_start_indices: Vec::from_slice(&[0]).unwrap(),
            is_done: false,
        })
    }

    /// Explores valid sequences, attempting to execute them on an I2C bus.
    pub fn explore<I2C, E, L>(
        &self,
        i2c: &mut I2C,
        executor: &mut E,
        logger: &mut L,
    ) -> Result<(), ExplorerError>
    where
        I2C: crate::compat::I2cCompat,
        E: CmdExecutor<I2C>,
        L: Logger,
    {
        let mut solved_addrs = [false; I2C_ADDRESS_COUNT];
        let mut num_solved_addrs = 0;
        let mut permutation_count = 0;

        let mut iter = self.permutations()?;
        logger.log_info("[explorer] Starting permutation exploration...");

        while let Some(sequence) = iter.next() {
            permutation_count += 1;
            let mut log_buf: String<LOG_BUFFER_CAPACITY> = String::new();
            if let Some(first_byte) = sequence.first().and_then(|b| b.first()) {
                let _ = writeln!(&mut log_buf, "[explorer] trying candidate starting with 0x{first_byte:02X}").ok();
            }

            for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                if solved_addrs[addr as usize] {
                    continue;
                }

                let all_ok = sequence
                    .iter()
                    .all(|&cmd| executor.exec(i2c, addr, cmd).is_ok());

                if all_ok {
                    let _ = writeln!(
                        &mut log_buf,
                        "[explorer] Success: Sequence works for addr 0x{addr:02X}"
                    )
                    .ok();
                    solved_addrs[addr as usize] = true;
                    num_solved_addrs += 1;
                }
            }
            logger.log_info(log_buf.as_str());
        }

        logger.log_info(&format!(
            "[explorer] Exploration complete. {num_solved_addrs} addresses solved across {permutation_count} permutations."
        ));

        if num_solved_addrs == 0 {
            Err(ExplorerError::NoValidAddressesFound)
        } else {
            Ok(())
        }
    }
}

impl<'a, const N: usize> Iterator for PermutationIter<'a, N> {
    type Item = Vec<&'a [u8], N>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }

        loop {
            // Check if we have a full permutation
            if self.current.len() + self.staged.len() == self.unresolved_indices.len() + self.staged.len() {
                // Return the full sequence by concatenating staged and current
                let mut full_sequence = self.staged.clone();
                full_sequence.extend_from_slice(&self.current).ok();
                
                // Backtrack to find the next permutation
                self.backtrack();
                return Some(full_sequence);
            }

            // Try to extend the current partial permutation
            if self.try_extend() {
                continue; // Extended, continue to next level of recursion
            } else {
                // Could not extend, backtrack
                if !self.backtrack() {
                    self.is_done = true;
                    return None; // No more permutations
                }
            }
        }
    }
}

impl<'a, const N: usize> PermutationIter<'a, N> {
    fn try_extend(&mut self) -> bool {
        let current_loop_start_idx = *self.loop_start_indices.last().unwrap();
        for (pos, &idx) in self.unresolved_indices.iter().enumerate().skip(current_loop_start_idx) {
            if self.used[pos] {
                continue;
            }

            let node = &self.unresolved_indices.get(pos).and_then(|&i| self.path_stack.get(i)).and_then(|&i| Some(&self.path_stack)).map_or(
                self.unresolved_indices.get(pos).and_then(|&i| self.path_stack.get(i)).and_then(|&i| Some(&self.path_stack)).map_or(
                    self.unresolved_indices.get(pos).and_then(|&i| self.path_stack.get(i)).and_then(|&i| Some(&self.path_stack)).map_or(
                        self.unresolved_indices.get(pos).and_then(|&i| self.path_stack.get(i)).and_then(|&i| Some(&self.path_stack)).map_or(
                            self.unresolved_indices.get(pos).and_then(|&i| Some(&self.unresolved_indices)),
                            |x| x
                        ),
                        |x| x
                    ),
                    |x| x
                ),
                |x| x
            );

            let node = &self.unresolved_indices.get(pos).and_then(|&i| Some(&self.sequence[i])).unwrap();

            let deps_satisfied = node.deps.iter().all(|&d| {
                if let Some(first_byte) = self.staged_and_current_set.get(d as usize) {
                    *first_byte
                } else {
                    false
                }
            });

            if deps_satisfied {
                self.current.push(node.bytes).unwrap();
                if let Some(first_byte) = node.bytes.first() {
                    self.staged_and_current_set[*first_byte as usize] = true;
                }
                self.used[pos] = true;
                self.path_stack.push(pos).unwrap();
                self.loop_start_indices.push(0).unwrap();
                return true;
            }
        }
        false
    }

    fn backtrack(&mut self) -> bool {
        if let Some(last_added_pos) = self.path_stack.pop() {
            let node = &self.unresolved_indices.get(last_added_pos).and_then(|&i| Some(&self.sequence[i])).unwrap();
            if let Some(first_byte) = node.bytes.first() {
                self.staged_and_current_set[*first_byte as usize] = false;
            }
            self.used[last_added_pos] = false;
            self.current.pop();

            self.loop_start_indices.pop();
            
            if let Some(last_loop_idx) = self.loop_start_indices.last_mut() {
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