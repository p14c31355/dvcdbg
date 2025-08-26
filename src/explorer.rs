// explorer.rs
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
use crate::compat::ascii;
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

/// Errors that can occur during command execution.
#[derive(Debug, PartialEq, Eq)]
pub enum ExecutorError {
    /// The command failed to execute due to an I2C error.
    I2cError(crate::error::ErrorKind),
    /// The command failed to execute (e.g., NACK, I/O error).
    ExecFailed,
    /// An internal buffer overflowed during command preparation.
    BufferOverflow,
}

/// Represents a single I2C command in the dependency graph.
///
/// The dependency is now on the index of the dependent command in the sequence.
#[derive(Copy, Clone)]
pub struct CmdNode<'a> {
    /// The I2C command bytes to be sent.
    pub bytes: &'a [u8],
    /// The indices of the commands that must precede this command.
    pub deps: &'a [usize],
}

/// A trait for executing a command on an I2C bus.
pub trait CmdExecutor<I2C> {
    /// Executes a given command byte sequence.
    fn exec(&mut self, i2c: &mut I2C, addr: u8, cmd: &[u8]) -> Result<(), ExecutorError>;
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
    
    fn log_error_fmt<F>(&mut self, fmt: F)
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

    fn log_error_fmt<F>(&mut self, _fmt: F)
    where
        F: FnOnce(&mut String<LOG_BUFFER_CAPACITY>) -> Result<(), core::fmt::Error>,
    {

    }
}

/// The core explorer, now a generic dependency graph manager.
pub struct Explorer<'a, const N: usize> {
    pub sequence: &'a [CmdNode<'a>],
}

/// An iterator that generates valid I2C command permutations.
pub struct PermutationIter<'a, const N: usize> {
    sequence: &'a [CmdNode<'a>],
    staged: Vec<&'a [u8], N>,
    unresolved_indices: Vec<usize, N>,
    current_permutation: Vec<&'a [u8], N>,
    used: Vec<bool, N>,
    used_indices: [bool; N], // Bitmask for O(1) checks
    path_stack: Vec<usize, N>,
    loop_start_indices: Vec<usize, N>, // Tracks search progress at each level
    is_done: bool,
}

/// The return type for the stage function.
type StageResult<'a, const N: usize> = Result<(Vec<&'a [u8], N>, Vec<usize, N>, [bool; N]), ExplorerError>;

pub struct ExploreResult {
    pub found_addrs: Vec<u8, I2C_ADDRESS_COUNT>,
    pub permutations_tested: usize,
}

impl<'a, const N: usize> Explorer<'a, N> {
    /// Performs an initial topological sort to stage commands without unresolved dependencies.
    fn stage(&self) -> StageResult<'a, N> {
        let mut in_degree = Vec::<usize, N>::new();
        in_degree.resize(self.sequence.len(), 0).map_err(|_| ExplorerError::TooManyCommands)?;

        // Calculate in-degrees for all nodes.
        // Build adjacency list for reverse dependencies (nodes that depend on current node)
// adj_list_rev[k] will contain a list of indices 'j' where self.sequence[j] depends on self.sequence[k]
let mut adj_list_rev: Vec<Vec<usize, N>, N> = Vec::new();
adj_list_rev.resize(self.sequence.len(), Vec::new()).map_err(|_| ExplorerError::BufferOverflow)?;

// Populate in-degrees and reverse adjacency list
for (i, node) in self.sequence.iter().enumerate() {
    in_degree[i] = node.deps.len();
    for &dep_idx in node.deps.iter() {
        // Basic validation for dependency index
        if dep_idx >= self.sequence.len() {
            return Err(ExplorerError::BufferOverflow); // Or a more specific error like InvalidDependencyIndex
        }
        adj_list_rev[dep_idx].push(i).map_err(|_| ExplorerError::BufferOverflow)?;
    }
}
        
        let mut queue = Vec::<usize, N>::new();
        for (i, &degree) in in_degree.iter().enumerate() {
            if degree == 0 {
                queue.push(i).map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        let mut staged: Vec<&'a [u8], N> = Vec::new();
        let mut staged_indices = [false; N];
        let mut staged_count = 0;
        let mut queue_idx = 0;
        
        while queue_idx < queue.len() {
            let idx = queue[queue_idx];
            queue_idx += 1;
            
            staged.push(self.sequence[idx].bytes).map_err(|_| ExplorerError::BufferOverflow)?;
            staged_indices[idx] = true;
            staged_count += 1;

             // For each node v that depends on u (which is 'idx' in this loop)
            for &v in adj_list_rev[idx].iter() {
                in_degree[v] -= 1;
                if in_degree[v] == 0 {
                    queue.push(v).map_err(|_| ExplorerError::BufferOverflow)?;
                }
            }
        }

        if staged_count != self.sequence.len() {
            return Err(ExplorerError::DependencyCycle);
        }
        
        let mut unresolved_indices = Vec::new();
        for (i, &is_staged) in staged_indices.iter().enumerate() {
            if !is_staged {
                unresolved_indices.push(i).map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }
        
        Ok((staged, unresolved_indices, staged_indices))
    }

    /// Returns a stack-safe iterator for all valid command permutations.
    pub fn permutations(&self) -> Result<PermutationIter<'a, N>, ExplorerError> {
        let (staged, unresolved_indices, staged_indices) = self.stage()?;

        let mut used = Vec::new();
        used.resize(unresolved_indices.len(), false).map_err(|_| ExplorerError::BufferOverflow)?;

        Ok(PermutationIter {
            sequence: self.sequence,
            staged,
            unresolved_indices,
            current_permutation: Vec::new(),
            used,
            used_indices: staged_indices,
            path_stack: Vec::new(),
            loop_start_indices: Vec::new(),
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
        let mut active_addrs: Vec<u8, I2C_ADDRESS_COUNT> = (I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END).collect();
        let mut found_addresses: Vec<u8, I2C_ADDRESS_COUNT> = Vec::new();
        let mut permutation_count = 0;

        let iter = self.permutations()?;
        logger.log_info("[explorer] Starting permutation exploration...\r\n");

        for sequence in iter {
            permutation_count += 1;
            
            let mut next_active_addrs: Vec<u8, I2C_ADDRESS_COUNT> = Vec::new();

            for &addr in active_addrs.iter() {
                let mut all_ok = true;
                for &cmd in sequence.iter() {
                    if let Err(e) = executor.exec(i2c, addr, cmd) {
                        all_ok = false;
                        logger.log_error_fmt(|buf| {
                            write!(buf, "[explorer] Execution failed for addr ")?;
                            ascii::write_bytes_hex_prefixed(buf, &[addr])?;
                            write!(buf, ": {:?}\r\n", e)?;
                            Ok(())
                        });
                        break;
                    }
                }
                if all_ok {
                    next_active_addrs.push(addr).map_err(|_| ExplorerError::BufferOverflow)?;
                }
            }
            active_addrs = next_active_addrs;
            if active_addrs.is_empty() {
                break;
            }
        }
        
        found_addresses.extend_from_slice(&active_addrs).map_err(|_| ExplorerError::BufferOverflow)?;

        logger.log_info_fmt(|buf| {
            writeln!(
                buf,
                "[explorer] Exploration complete. {} addresses found across {} permutations.",
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
            if self.current_permutation.len() + self.staged.len() == self.sequence.len() {
                // Construct the full sequence by concatenating staged and current
                let mut full_sequence = self.staged.clone();
                full_sequence.extend_from_slice(&self.current_permutation).unwrap_or_else(|_| unreachable!());
                
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
        let last_pos = self.loop_start_indices.last().copied().unwrap_or(0);
        for pos in last_pos..self.unresolved_indices.len() {
            let idx = self.unresolved_indices[pos];

            if self.used[pos] {
                continue;
            }

            let node = &self.sequence[idx];

            let deps_satisfied = node.deps.iter().all(|&d| self.used_indices[d]);

            if deps_satisfied {
                self.current_permutation.push(node.bytes).unwrap();
                self.used_indices[idx] = true;
                self.used[pos] = true;
                self.path_stack.push(pos).unwrap();
                self.loop_start_indices.push(pos + 1).unwrap();
                return true;
            }
        }
        false
    }

    fn backtrack(&mut self) -> bool {
        if let Some(last_added_pos) = self.path_stack.pop() {
            let node_idx = self.unresolved_indices[last_added_pos];
            self.used_indices[node_idx] = false;
            
            self.used[last_added_pos] = false;
            self.current_permutation.pop();
            self.loop_start_indices.pop();

            true
        } else {
            false
        }
    }
}