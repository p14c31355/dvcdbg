// explorer.rs
//! # I2C Command Sequence Explorer
//!
//! This module provides an algorithm and supporting utilities for discovering
//! valid sequences of I2C commands when device dependencies are unknown or
//! partially specified. It is designed for embedded bring-up scenarios, where
//! experimenting with permutations of initialization commands can reveal the
//! correct sequence for a new or undocumented device.
//!
//! ## Overview
//! - [`Explorer`] manages a dependency graph of commands and produces only
//!   valid permutations that satisfy the declared constraints.
//! - [`PermutationIter`] generates these permutations using an iterative,
//!   stack-safe backtracking algorithm (no recursion).
//! - [`CmdExecutor`] abstracts the execution of a command on the I2C bus.
//! - [`Logger`] provides pluggable logging backends (serial console, null logger, etc.).
//!
//! ## Key Features
//! 1. **Separation of Concerns**: The permutation engine (`PermutationIter`) is
//!    isolated from bus execution logic (`explore`).
//! 2. **Iterator-based API**: The [`Explorer::permutations`] method yields an
//!    iterator, making the algorithm testable and composable.
//! 3. **Generic Capacity**: The const generic `N` defines the maximum command
//!    capacity at compile time, enabling efficient use on resource-constrained
//!    microcontrollers.
//! 4. **Flexible Logging**: The [`Logger`] trait supports both formatted and
//!    lightweight logging without allocating large buffers.
//! 5. **Robust Error Handling**: [`ExplorerError`] reports dependency cycles,
//!    buffer exhaustion, and runtime execution issues explicitly.
//!
//! ## Typical Use Case
//! - Bring-up of a new I2C peripheral when the required initialization sequence
//!   is undocumented or incomplete.
//! - Automated discovery of valid command orderings under dependency constraints.
//! - Filtering of device addresses that respond consistently to a tested sequence.
//!
//! ## Example
//! ```ignore
//! use dvcdbg::prelude::*;
//! use heapless::Vec;
//!
//! // Example executor for a specific I2C implementation.
//! struct MyExecutor;
//! impl<I2C: crate::compat::I2cCompat> CmdExecutor<I2C> for MyExecutor {
//!     fn exec(
//!         &mut self,
//!         i2c: &mut I2C,
//!         addr: u8,
//!         cmd: &[u8]
//!     ) -> Result<(), crate::explorer::ExecutorError> {
//!         // Simplified example: prepend 0x00 control byte.
//!         if cmd.len() != 1 {
//!             return Err(crate::explorer::ExecutorError::ExecFailed);
//!         }
//!         let buf = [0x00, cmd[0]];
//!         i2c.write(addr, &buf)
//!             .map_err(|e| crate::explorer::ExecutorError::I2cError(e.to_compat(Some(addr))))
//!     }
//! }
//!
//! // Dummy logger that ignores messages.
//! struct NullLogger;
//! impl Logger for NullLogger {
//!     fn log_info(&mut self, _msg: &str) {}
//!     fn log_warning(&mut self, _msg: &str) {}
//!     fn log_error(&mut self, _msg: &str) {}
//!     fn log_info_fmt<F>(&mut self, _fmt: F)
//!     where F: FnOnce(&mut heapless::String<{ crate::explorer::LOG_BUFFER_CAPACITY }>) -> Result<(), core::fmt::Error> {}
//!     fn log_error_fmt<F>(&mut self, _fmt: F)
//!     where F: FnOnce(&mut heapless::String<{ crate::explorer::LOG_BUFFER_CAPACITY }>) -> Result<(), core::fmt::Error> {}
//! }
//!
//! // Define candidate commands with dependencies.
//! const CAPACITY: usize = 32;
//! let cmds = &[
//!     CmdNode { bytes: &[0x01], deps: &[] },
//!     CmdNode { bytes: &[0x02], deps: &[0] }, // depends on first command
//!     CmdNode { bytes: &[0x03], deps: &[0] },
//! ];
//!
//! let explorer = Explorer::<CAPACITY> { sequence: cmds };
//! let mut executor = MyExecutor;
//! let mut logger = NullLogger;
//! let mut i2c = /* platform-specific I2C impl */;
//!
//! let result = explorer.explore(&mut i2c, &mut executor, &mut logger);
//! if let Err(e) = result {
//!     logger.log_error(&format!("Exploration failed: {:?}", e));
//! }
//! ```

use crate::compat::ascii;
use core::fmt::Write;
use heapless::Vec;

use crate::scanner::{I2C_SCAN_ADDR_END, I2C_SCAN_ADDR_START};
const I2C_ADDRESS_COUNT: usize = 128;
pub const LOG_BUFFER_CAPACITY: usize = 1024;

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
    /// A dependency index is out of bounds.
    InvalidDependencyIndex,
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

impl From<ExecutorError> for ExplorerError {
    fn from(error: ExecutorError) -> Self {
        match error {
            ExecutorError::I2cError(_) => ExplorerError::ExecutionFailed,
            ExecutorError::ExecFailed => ExplorerError::ExecutionFailed,
            ExecutorError::BufferOverflow => ExplorerError::BufferOverflow,
        }
    }
}

/// Represents a single I2C command in the dependency graph.
///
/// The dependency is now on the index of the dependent command in the sequence.
#[derive(Clone)]
pub struct CmdNode<'a> {
    /// The I2C command bytes to be sent.
    pub bytes: &'a [u8],
    /// The indices of the commands that must precede this command.
    pub deps: &'a [usize],
}

/// A trait for executing a command on an I2C bus.
pub trait CmdExecutor<I2C> {
    /// Executes a given command byte sequence.
    fn exec<S>(
        &mut self,
        i2c: &mut I2C,
        addr: u8,
        cmd: &[u8],
        logger: &mut S,
    ) -> Result<(), ExecutorError>
    where
        S: core::fmt::Write + crate::logger::Logger;
}

/// The core explorer, now a generic dependency graph manager.
pub struct Explorer<'a, const N: usize> {
    pub sequence: &'a [CmdNode<'a>],
}

/// An iterator that generates valid I2C command permutations (topological sorts).
///
/// This iterator uses an iterative backtracking approach to find all possible
/// valid sequences of commands, respecting their dependencies.
pub struct PermutationIter<'a, const N: usize> {
    sequence: &'a [CmdNode<'a>],
    current_permutation: Vec<&'a [u8], N>,
    used: [bool; N], // Tracks which original command indices are currently in `current_permutation`
    in_degree: Vec<usize, N>, // Current in-degrees, updated dynamically during permutation generation
    adj_list_rev: Vec<Vec<usize, N>, N>, // Reverse adjacency list: adj_list_rev[i] contains nodes that depend on node 'i'
    path_stack: Vec<usize, N>, // Stack of original command indices added to `current_permutation`
    loop_start_indices: Vec<usize, N>, // Tracks search progress at each level of the permutation tree
    is_done: bool,
    total_nodes: usize,
}

pub struct ExploreResult {
    pub found_addrs: Vec<u8, I2C_ADDRESS_COUNT>,
    pub permutations_tested: usize,
}

impl<'a, const N: usize> Explorer<'a, N> {
    /// Returns a stack-safe iterator for all valid command permutations (topological sorts).
    ///
    /// This function first performs cycle detection using a modified Kahn's algorithm.
    /// If a cycle is detected, it returns an `ExplorerError::DependencyCycle`.
    /// Otherwise, it initializes and returns a `PermutationIter` to generate all valid permutations.
    pub fn permutations(&self) -> Result<PermutationIter<'a, N>, ExplorerError> {
        if self.sequence.len() > N {
            return Err(ExplorerError::TooManyCommands);
        }

        let mut initial_in_degree = Vec::<usize, N>::new();
        initial_in_degree
            .resize(self.sequence.len(), 0)
            .map_err(|_| ExplorerError::BufferOverflow)?;

        let mut adj_list_rev: Vec<Vec<usize, N>, N> = Vec::new();
        adj_list_rev
            .resize(self.sequence.len(), Vec::new())
            .map_err(|_| ExplorerError::BufferOverflow)?;

        for (i, node) in self.sequence.iter().enumerate() {
            // The in-degree of a node is the number of dependencies it has.
            initial_in_degree[i] = node.deps.len();
            for &dep_idx in node.deps.iter() {
                if dep_idx >= self.sequence.len() {
                    return Err(ExplorerError::InvalidDependencyIndex);
                }
                // Add 'i' to the list of nodes that depend on 'dep_idx'
                adj_list_rev[dep_idx]
                    .push(i)
                    .map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        // Cycle detection using a modified Kahn's algorithm
        let mut temp_in_degree = initial_in_degree.clone();
        let mut q = Vec::<usize, N>::new();
        for i in 0..self.sequence.len() {
            if temp_in_degree[i] == 0 {
                q.push(i).map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        let mut count = 0;
        let mut q_idx = 0;
        while q_idx < q.len() {
            let u = q[q_idx];
            q_idx += 1;
            count += 1;

            for &v in adj_list_rev[u].iter() {
                temp_in_degree[v] -= 1;
                if temp_in_degree[v] == 0 {
                    q.push(v).map_err(|_| ExplorerError::BufferOverflow)?;
                }
            }
        }

        if count != self.sequence.len() {
            return Err(ExplorerError::DependencyCycle);
        }

        Ok(PermutationIter {
            sequence: self.sequence,
            current_permutation: Vec::new(),
            used: [false; N], // Initialize all to false
            in_degree: initial_in_degree,
            adj_list_rev,
            path_stack: Vec::new(),
            loop_start_indices: Vec::new(), // Start empty, will be pushed to
            is_done: false,
            total_nodes: self.sequence.len(),
        })
    }

    /// Explores valid sequences, attempting to execute them on an I2C bus.
    ///
    /// This function iterates through all valid command permutations generated by `PermutationIter`,
    /// and for each permutation, attempts to execute it on all active I2C addresses.
    /// It filters out addresses that fail to respond to any command in a sequence.
    ///
    /// # Parameters
    /// - `i2c`: An I2C implementation used to test candidate sequences against device addresses.
    /// - `executor`: The object responsible for executing a single command on the bus.
    /// - `logger`: The object responsible for logging progress and results.
    ///
    /// # Returns
    /// - `Ok(ExploreResult)` containing the list of found addresses and the number of permutations tested.
    /// - `Err(ExplorerError)` if an error occurs during permutation generation or I2C execution.
    pub fn explore<I2C, E, L>(
        &self,
        i2c: &mut I2C,
        executor: &mut E,
        logger: &mut L,
    ) -> Result<ExploreResult, ExplorerError>
    where
        I2C: crate::compat::I2cCompat,
        E: CmdExecutor<I2C>,
        L: crate::logger::Logger + core::fmt::Write,
    {
        // Handle the case where no commands are provided.
        // An empty sequence means there's nothing to explore,
        // so no valid addresses can be found through this exploration process.
        if self.sequence.is_empty() {
            logger.log_info(
                "[explorer] No commands provided for exploration. Returning no valid addresses.",
            );
            return Err(ExplorerError::NoValidAddressesFound);
        }
        let mut found_addresses: Vec<u8, I2C_ADDRESS_COUNT> = Vec::new();
        let mut solved_addrs = [false; I2C_ADDRESS_COUNT];
        let mut permutation_count = 0;

        let iter = self.permutations()?;
        logger.log_info("[explorer] Starting permutation exploration...");

        for sequence in iter {
            permutation_count += 1;

            for addr_val in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                let addr_idx = addr_val as usize;
                if solved_addrs[addr_idx] {
                    continue;
                }

                let mut all_ok = true;
                for &cmd in sequence.iter() {
                    if let Err(e) = executor.exec(i2c, addr_val, cmd, logger) {
                        // Pass logger to executor.exec
                        all_ok = false;
                        logger.log_error_fmt(|buf| {
                            write!(buf, "[explorer] Execution failed for addr ")?;
                            ascii::write_bytes_hex_fmt(buf, &[addr_val])?;
                            write!(buf, ": {e:?}\r\n")?;
                            Ok(())
                        });
                        break;
                    }
                }

                if all_ok {
                    if found_addresses.push(addr_val).is_ok() {
                        solved_addrs[addr_idx] = true;
                    } else {
                        return Err(ExplorerError::BufferOverflow);
                    }
                }
            }

            // Optimization: if all possible addresses have been found, we can stop.
            if found_addresses.len() == (I2C_SCAN_ADDR_END - I2C_SCAN_ADDR_START + 1) as usize {
                break;
            }
        }

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

    /// Generates a single valid topological sort of the command sequence.
    /// This is useful when only one valid ordering is needed, and avoids
    /// the computational cost of generating all permutations.
    ///
    /// Returns `Ok(Vec<&'a [u8], N>)` containing one valid command sequence,
    /// or `Err(ExplorerError)` if a cycle is detected or buffer overflows.
    pub fn get_one_topological_sort<'a, const N: usize>(
        sequence: &[CmdNode<'a>; N],
        serial: &mut impl core::fmt::Write,
    ) -> Result<[&'a [u8]; N], ExplorerError> {
        let len = sequence.len();

        // Initialize in-degree array
        let mut in_degree: [usize; N] = [0; N];

        // reversed adjacency list & length
        let mut adj_list_rev: [[usize; N]; N] = [[0; N]; N];
        let mut adj_list_len: [usize; N] = [0; N];

        // Define result sequence
        let mut result_sequence: [&[u8]; N] = [&[]; N];
        let mut result_len = 0;

        // Topological sort core
        for (i, node) in sequence.iter().enumerate() {
            in_degree[i] = node.deps.len();
            for &dep_idx in node.deps.iter() {
                if dep_idx >= len {
                    writeln!(serial, "[error] Node {} has invalid dep index {} (len={})", i, dep_idx, len).ok();
                    return Err(ExplorerError::InvalidDependencyIndex);
                }
                let pos = adj_list_len[dep_idx];
                if pos >= N {
                    return Err(ExplorerError::BufferOverflow);
                }
                adj_list_rev[dep_idx][pos] = i;
                adj_list_len[dep_idx] += 1;
            }
        }

        // Initialize queue with nodes having zero in-degree
        let mut q: [usize; N] = [0; N];
        let mut head = 0;
        let mut tail = 0;
        for i in 0..len {
            if in_degree[i] == 0 {
                q[tail] = i;
                tail += 1;
            }
        }

        // sort
        while head < tail {
            let u = q[head];
            head += 1;

            result_sequence[result_len] = sequence[u].bytes;
            result_len += 1;

            for i in 0..adj_list_len[u] {
                let v = adj_list_rev[u][i];
                in_degree[v] -= 1;
                if in_degree[v] == 0 {
                    q[tail] = v;
                    tail += 1;
                }
            }
        }

        if result_len != len {
            writeln!(serial, "[error] Dependency cycle detected").ok();
            return Err(ExplorerError::DependencyCycle);
        }

        Ok(result_sequence)
    }
}

impl<'a, const N: usize> Iterator for PermutationIter<'a, N> {
    type Item = Vec<&'a [u8], N>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }

        loop {
            // If we have a complete permutation, return it and prepare for the next one.
            if self.current_permutation.len() == self.total_nodes {
                let full_sequence = self.current_permutation.clone();
                // Backtrack to find the next permutation
                if !self.backtrack() {
                    self.is_done = true;
                }
                return Some(full_sequence);
            }

            // Try to extend the current partial permutation
            if self.try_extend() {
                // Successfully extended, continue building the permutation
                continue;
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
    /// Attempts to extend the current partial permutation by adding a new command.
    ///
    /// It iterates through all available (not yet used) commands and checks if their
    /// dependencies are satisfied (i.e., their in-degree is 0). If a valid command
    /// is found, it's added to the current permutation, its `used` status is updated,
    /// and the in-degrees of its dependent nodes are decremented.
    ///
    /// Returns `true` if a command was successfully added, `false` otherwise.
    fn try_extend(&mut self) -> bool {
        // The `loop_start_indices` tracks the starting point for the search at the current depth.
        // If it's empty, we start from the beginning (0). Otherwise, we continue from where we left off.
        let start_idx_for_level = self.loop_start_indices.last().copied().unwrap_or(0);

        // Iterate through all possible nodes (0 to total_nodes-1)
        for idx in start_idx_for_level..self.total_nodes {
            // If this node is already used in the current permutation, skip it
            if self.used[idx] {
                continue;
            }

            // Check if this node's dependencies are satisfied (i.e., its current in-degree is 0)
            if self.in_degree[idx] == 0 {
                // Make choice: Add this node to the current permutation
                // Note: unwrap() is used here assuming N is sufficiently large based on initial checks.
                self.current_permutation
                    .push(self.sequence[idx].bytes)
                    .unwrap();
                self.used[idx] = true; // Mark as used

                // Decrement in-degrees for all nodes that depend on this one
                for &dependent_idx in self.adj_list_rev[idx].iter() {
                    self.in_degree[dependent_idx] -= 1;
                }

                self.path_stack.push(idx).unwrap(); // Push the original index of the command
                // Store the next starting point for this level (for backtracking to this level)
                self.loop_start_indices.push(idx + 1).unwrap();
                return true; // Successfully extended
            }
        }
        false // No node found to extend the current permutation
    }

    /// Backtracks to the previous decision point in the permutation search.
    ///
    /// This method undoes the last choice made: it removes the last added command
    /// from the current permutation, unmarks it as used, and increments the
    /// in-degrees of its dependent nodes (reversing the decrement).
    /// It then updates the `loop_start_indices` to ensure the next search
    /// at the parent level continues from the next sibling.
    ///
    /// Returns `true` if backtracking can continue (i.e., there are more options
    /// at a previous level), or `false` if the root was reached and no more
    /// permutations can be generated.
    fn backtrack(&mut self) -> bool {
        if let Some(last_added_idx) = self.path_stack.pop() {
            // Undo the choice: Remove the last added node from the permutation
            self.current_permutation.pop();
            self.used[last_added_idx] = false; // Unmark as used

            // Increment in-degrees for all nodes that depend on this one (undo decrement)
            for &dependent_idx in self.adj_list_rev[last_added_idx].iter() {
                self.in_degree[dependent_idx] += 1;
            }

            // Pop the loop start for the level we just finished. The next search start for the parent
            // was already set when this level was pushed.
            self.loop_start_indices.pop();

            // If path_stack is empty after pop, we've backtracked past the root
            if self.path_stack.is_empty() {
                self.is_done = true;
                return false;
            }
            true
        } else {
            // Already at the root and no more options
            self.is_done = true;
            false
        }
    }
}
