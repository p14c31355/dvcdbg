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
//! use heapless::Vec;
//!
//! // Example implementations for a specific platform
//! struct MyExecutor;
//! impl<I2C: I2cCompat> CmdExecutor<I2C> for MyExecutor {
//!     fn exec(&mut self, i2c: &mut I2C, addr: u8, cmd: &[u8]) -> Result<(), ()> {
//!         // This example executor only supports single-byte commands for simplicity,
//!         // and assumes a device protocol that requires a 0x00 control byte.
//!         if cmd.len() != 1 {
//!             return Err(());
//!         }
//!         let buf = [0x00, cmd[0]];
//!         i2c.write(addr, &buf).map_err(|_| ())
//!     }
//! }
//! struct NullLogger;
//! impl Logger for NullLogger {
//!     fn log_info(&mut self, _msg: &str) {}
//!     fn log_warning(&mut self, _msg: &str) {}
//!     fn log_error(&mut self, _msg: &str) {}
//! }
//!
//! // Define commands with dependencies
//! const CAPACITY: usize = 32;
//! let cmds = &[
//!     CmdNode { bytes: &[0x01], deps: &[] },
//!     CmdNode { bytes: &[0x02], deps: &[0] }, // Depends on the first command at index 0
//!     CmdNode { bytes: &[0x03], deps: &[0] },
//! ];
//!
//! // The generic parameter 32 matches the capacity of the Vecs
//! let explorer = Explorer::<CAPACITY> { sequence: cmds };
//! let mut executor = MyExecutor;
//! let mut logger = NullLogger;
//! let mut i2c = // ...
//!
//! let result = explorer.explore(&mut i2c, &mut executor, &mut logger);
//! if let Err(e) = result {
//!     // The logger here would be a more verbose one, e.g., one that prints to a serial console
//!     logger.log_error(&format!("Exploration failed: {:?}", e));
//! }
//! ```

use core::fmt::Write;
use heapless::{String, Vec};

const I2C_SCAN_ADDR_END: u8 = 127;
const I2C_SCAN_ADDR_START: u8 = 1;
const I2C_ADDRESS_COUNT: usize = 128;
pub const LOG_BUFFER_CAPACITY: usize = 512;

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
    /// An internal buffer overflowed.
    BufferOverflow,
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

    /// Logs formatted information efficiently, by writing directly to an internal buffer.
    fn log_info_fmt<F>(&mut self, fmt: F)
    where
        F: FnOnce(&mut String<LOG_BUFFER_CAPACITY>) -> Result<(), core::fmt::Error>;
}

// Dummy logger for platforms without console output
pub struct NullLogger;
impl Logger for NullLogger {
    fn log_info(&mut self, _msg: &str) {}
    fn log_warning(&mut self, _msg: &str) {}
    fn log_error(&mut self, _msg: &str) {}
    fn log_info_fmt<F>(&mut self, _fmt: F)
    where
        F: FnOnce(&mut String<LOG_BUFFER_CAPACITY>) -> Result<(), core::fmt::Error>,
    {
    }
}

/// The core explorer, now a generic dependency graph manager.
pub struct Explorer<'a, const N: usize> {
    pub sequence: &'a [CmdNode<'a, N>],
}

/// An iterator that generates valid I2C command permutations.
pub struct PermutationIter<'a, const N: usize> {
    sequence: &'a [CmdNode<'a, N>],
    staged: Vec<&'a [u8], N>,
    unresolved_indices: Vec<usize, N>,
    current: Vec<&'a [u8], N>,
    used: [bool; N],
    staged_and_current_indices: Vec<usize, N>,
    path_stack: Vec<usize, N>,
    loop_start_indices: Vec<usize, N>,
    is_done: bool,
}

pub struct ExploreResult {
    pub found_addrs: Vec<u8, I2C_ADDRESS_COUNT>,
    pub permutations_tested: usize,
}

impl<'a, const N: usize> Explorer<'a, N> {
    /// Performs an initial topological sort to stage commands without unresolved dependencies.
    fn stage(&self) -> Result<(Vec<&'a [u8], N>, Vec<usize, N>), ExplorerError> {
        if self.sequence.len() > N {
            return Err(ExplorerError::TooManyCommands);
        }
        let mut staged: Vec<&'a [u8], N> = Vec::new();
        let mut remaining: Vec<usize, N> = (0..self.sequence.len()).collect();
        let mut staged_indices = Vec::<usize, N>::new();

        loop {
            let before = staged.len();
            let mut i = 0;
            while i < remaining.len() {
                let node_idx = remaining[i];
                let node = &self.sequence[node_idx];
                let deps_satisfied = node
                    .deps
                    .iter()
                    .all(|&d| staged_indices.contains(&d));

                if deps_satisfied {
                    staged.push(node.bytes).map_err(|_| ExplorerError::TooManyCommands)?;
                    staged_indices.push(node_idx).map_err(|_| ExplorerError::TooManyCommands)?;
                    remaining.swap_remove(i);
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

        let mut staged_and_current_indices = Vec::new();
        for i in staged_indices {
            staged_and_current_indices.push(i).map_err(|_| ExplorerError::BufferOverflow)?;
        }
        
        let mut used = [false; N];
        if unresolved_indices.len() > N { return Err(ExplorerError::TooManyCommands) }

        Ok(PermutationIter {
            sequence: self.sequence,
            staged,
            unresolved_indices,
            current: Vec::new(),
            used,
            staged_and_current_indices,
            path_stack: Vec::new(),
            loop_start_indices: Vec::from_slice(&[0]).map_err(|_| ExplorerError::TooManyCommands)?,
            is_done: false,
        })
    }
    
    /// Explores valid sequences, attempting to execute them on an I2C bus.
    pub fn explore<I2C, E, L>(
        &self,
        i2c: &mut I2C,
        executor: &mut E,
        logger: &mut L,
    ) -> Result<ExploreResult, ExplorerError>
    where
        I2C: crate::compat::I2cCompat,
        E: CmdExecutor<I2C>,
        L: Logger,
    {
        let mut solved_addrs = [false; I2C_ADDRESS_COUNT];
        let mut found_addresses: Vec<u8, I2C_ADDRESS_COUNT> = Vec::new();
        let mut permutation_count = 0;

        let mut iter = self.permutations()?;
        logger.log_info("[explorer] Starting permutation exploration...");

        while let Some(sequence) = iter.next() {
            permutation_count += 1;

            for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                if solved_addrs[addr as usize] {
                    continue;
                }

                let all_ok = sequence
                    .iter()
                    .all(|&cmd| executor.exec(i2c, addr, cmd).is_ok());

                if all_ok {
                    logger.log_info_fmt(|buf| {
                        writeln!(
                            buf,
                            "[explorer] Success: Sequence works for addr 0x{:02X}",
                            addr
                        )
                    });
                    solved_addrs[addr as usize] = true;
                    found_addresses.push(addr).map_err(|_| ExplorerError::BufferOverflow)?;
                }
            }
        }
        
        logger.log_info_fmt(|buf| {
            writeln!(
                buf,
                "[explorer] Exploration complete. {} addresses solved across {} permutations.",
                found_addresses.len(),
                permutation_count
            )
        });


        if found_addresses.is_empty() {
            Err(ExplorerError::NoValidAddressesFound)
        } else {
            Ok(ExploreResult {
                found_addrs: found_addresses,
                permutations_tested: permutation_count,
            })
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
            if self.current.len() + self.staged.len() == self.sequence.len() {
                // Return the full sequence by concatenating staged and current
                let mut full_sequence = self.staged.clone();
                full_sequence.extend_from_slice(&self.current).unwrap_or_else(|_| unreachable!());
                
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

            let node = &self.sequence[idx];

            let deps_satisfied = node.deps.iter().all(|&d| {
                self.staged_and_current_indices.contains(&d)
            });

            if deps_satisfied {
                self.current.push(node.bytes).unwrap();
                self.staged_and_current_indices.push(idx).unwrap();
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
            let node_idx = self.unresolved_indices[last_added_pos];
            let remove_idx = self.staged_and_current_indices.iter().position(|&x| x == node_idx).unwrap();
            self.staged_and_current_indices.swap_remove(remove_idx);
            
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