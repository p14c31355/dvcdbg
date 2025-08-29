// explorer.rs
use crate::error::{ExecutorError, ExplorerError};
use crate::compat::err_compat::HalErrorExt;
use core::fmt::Write;

use crate::scanner::{I2C_SCAN_ADDR_END, I2C_SCAN_ADDR_START};
use heapless::Vec;
const I2C_ADDRESS_COUNT: usize = 128;

#[derive(Copy, Clone)]
pub struct CmdNode {
    pub bytes: &'static [u8],
    pub deps: &'static [usize],
}

pub trait CmdExecutor<I2C, const BUF_CAP: usize> {
    fn exec<S>(
        &mut self,
        i2c: &mut I2C,
        addr: u8,
        cmd: &[u8],
        logger: &mut S,
    ) -> Result<(), ExecutorError>
    where
        S: core::fmt::Write + crate::explore::logger::Logger<BUF_CAP>;
}

/// A command executor that prepends a prefix to each command.
pub struct PrefixExecutor<const BUF_CAP: usize> {
    prefix: u8,
    init_sequence: heapless::Vec<u8, 64>,
    initialized_addrs: [bool; 128],
    buffer: heapless::Vec<u8, BUF_CAP>,
}

impl<const BUF_CAP: usize> PrefixExecutor<BUF_CAP> {
    pub fn new(prefix: u8, init_sequence: heapless::Vec<u8, 64>) -> Self {
        Self {
            prefix,
            init_sequence,
            initialized_addrs: [false; 128],
            buffer: heapless::Vec::new(),
        }
    }
}

impl<I2C, const BUF_CAP: usize> crate::explore::explorer::CmdExecutor<I2C, BUF_CAP>
    for PrefixExecutor<BUF_CAP>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
{
    fn exec<S>(
        &mut self,
        i2c: &mut I2C,
        addr: u8,
        cmd: &[u8],
        logger: &mut S,
    ) -> Result<(), ExecutorError>
    where
        S: core::fmt::Write + crate::explore::logger::Logger<BUF_CAP>,
    {
        fn short_delay() {
            for _ in 0..8_000 {
                core::hint::spin_loop();
            }
        }

        let addr_idx = addr as usize;

        if !self.initialized_addrs[addr_idx] && !self.init_sequence.is_empty() {
            logger
                .log_info_fmt(|buf| writeln!(buf, "[Info] I2C initializing for 0x{addr:02X}..."));

            for &c in self.init_sequence.iter() {
                let command = [self.prefix, c];
                let mut ok = false;

                for _attempt in 0..10 {
                    match i2c.write(addr, &command) {
                        Ok(_) => {
                            ok = true;
                            break;
                        }
                        Err(e) => {
                            let compat_err = e.to_compat(Some(addr));
                            logger.log_error_fmt(|buf| {
                                writeln!(buf, "[I2C retry error] {compat_err:?}")
                            });
                            short_delay();
                        }
                    }
                }

                if !ok {
                    return Err(ExecutorError::I2cError(
                        crate::error::ErrorKind::I2c(crate::error::I2cError::Nack),
                    ));
                }
                short_delay();
            }

            self.initialized_addrs[addr_idx] = true;
            logger.log_info_fmt(|buf| writeln!(buf, "[Info] I2C initialized for 0x{addr:02X}"));
        }

        self.buffer.clear();
        self.buffer
            .push(self.prefix)
            .map_err(|_| ExecutorError::BufferOverflow)?;
        self.buffer
            .extend_from_slice(cmd)
            .map_err(|_| ExecutorError::BufferOverflow)?;

        for _ in 0..10 {
            match i2c.write(addr, &self.buffer) {
                Ok(_) => {
                    short_delay();
                    return Ok(());
                }
                Err(e) => {
                    let compat_err = e.to_compat(Some(addr));
                    logger.log_error_fmt(|buf| writeln!(buf, "[I2C retry error] {compat_err:?}"));
                    short_delay();
                }
            }
        }

        Err(ExecutorError::I2cError(
            crate::error::ErrorKind::I2c(crate::error::I2cError::Nack),
        ))
    }
}


pub struct Explorer<'a, const N: usize> {
    pub sequence: &'a [CmdNode],
}

pub struct ExploreResult {
    pub found_addrs: Vec<u8, I2C_ADDRESS_COUNT>,
    pub permutations_tested: usize,
}

impl<'a, const N: usize> Explorer<'a, N> {
    pub const fn max_cmd_len(&self) -> usize {
        let mut max_len = 0;
        let mut i = 0;
        while i < N {
            let len = self.sequence[i].bytes.len();
            if len > max_len {
                max_len = len;
            }
            i += 1;
        }
        max_len + 1 // prefix add
    }
}

impl<'a, const N: usize> Explorer<'a, N> {
    pub fn explore<I2C, E, L, const BUF_CAP: usize>(
        &self,
        i2c: &mut I2C,
        executor: &mut E,
        logger: &mut L,
    ) -> Result<ExploreResult, ExplorerError>
    where
        I2C: crate::compat::I2cCompat,
        E: CmdExecutor<I2C, BUF_CAP>,
        L: crate::explore::logger::Logger<BUF_CAP> + core::fmt::Write,
    {
        if self.sequence.is_empty() {
            logger.log_info("[explorer] No commands provided.");
            return Err(ExplorerError::NoValidAddressesFound);
        }

        let mut found_addresses: Vec<u8, I2C_ADDRESS_COUNT> = Vec::new();
        let mut solved_addrs: [bool; I2C_ADDRESS_COUNT] = [false; I2C_ADDRESS_COUNT];
        let mut permutation_count = 0;
        let iter = PermutationIter::new(self)?;
        logger.log_info("[explorer] Starting permutation exploration...");
        for sequence in iter {
            permutation_count += 1;
            for addr_val in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                let addr_idx = addr_val as usize;
                if solved_addrs[addr_idx] {
                    continue;
                }
                let mut all_ok = true;
                for &cmd_bytes in sequence.iter() {
                    if let Err(e) = executor.exec(i2c, addr_val, cmd_bytes, logger) {
                        all_ok = false;
                        logger.log_error_fmt(|buf| {
                            use core::fmt::Write;
                            let _ = writeln!(
                                buf,
                                "[explorer] Execution failed for addr 0x{:02X}: {:?}",
                                addr_val, e
                            );
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
            use core::fmt::Write;
            let _ = writeln!(
                buf,
                "[explorer] Exploration complete. {} addresses found across {} permutations.",
                found_addresses.len(),
                permutation_count
            );
            Ok(())
        });

        if found_addresses.is_empty() {
            return Err(ExplorerError::NoValidAddressesFound);
        }

        Ok(ExploreResult {
            found_addrs: found_addresses,
            permutations_tested: permutation_count,
        })
    }
    /// Generates a single valid topological sort of the command sequence.
    /// This is useful when only one valid ordering is needed, and avoids
    /// the computational cost of generating all permutations.
    ///
    /// Returns `Ok(Vec<&'a [u8], N>)` containing one valid command sequence,
    /// or `Err(ExplorerError)` if a cycle is detected or buffer overflows.
    pub fn get_one_topological_sort_buf<const MAX_CMD_LEN: usize>(
        &self,
        _serial: &mut impl core::fmt::Write,
        failed_nodes: &[bool; N],
    ) -> Result<(heapless::Vec<heapless::Vec<u8, MAX_CMD_LEN>, N>, heapless::Vec<usize, N>), ExplorerError> {
        let len = self.sequence.len();
        let mut in_degree: heapless::Vec<usize, N> = heapless::Vec::new();
        in_degree.resize(len, 0).map_err(|_| ExplorerError::BufferOverflow)?;
        let mut adj_list_rev: heapless::Vec<heapless::Vec<usize, N>, N> = heapless::Vec::new();
        adj_list_rev
            .resize(len, heapless::Vec::new())
            .map_err(|_| ExplorerError::BufferOverflow)?;

        for (i, node) in self.sequence.iter().enumerate() {
            if failed_nodes[i] {
                continue;
            }
            in_degree[i] = node.deps.len();
            for &dep_idx in node.deps.iter() {
                if dep_idx >= len {
                    return Err(ExplorerError::InvalidDependencyIndex);
                }
                adj_list_rev[dep_idx]
                    .push(i)
                    .map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        let mut q: heapless::Vec<usize, N> = heapless::Vec::new();
        for (i, &degree) in in_degree.iter().enumerate().take(len) {
            if !failed_nodes[i] && degree == 0 {
                q.push(i).map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        let mut result_sequence: heapless::Vec<heapless::Vec<u8, MAX_CMD_LEN>, N> =
            heapless::Vec::new();
        let mut result_len_per_node: heapless::Vec<usize, N> = heapless::Vec::new();
        let mut visited_count = 0;

        while let Some(u) = q.pop() {
            visited_count += 1;

            let cmd_bytes = self.sequence[u].bytes;
            let mut cmd_vec: heapless::Vec<u8, MAX_CMD_LEN> = heapless::Vec::new();
            cmd_vec
                .extend_from_slice(cmd_bytes)
                .map_err(|_| ExplorerError::BufferOverflow)?;
            result_len_per_node
                .push(cmd_vec.len())
                .map_err(|_| ExplorerError::BufferOverflow)?;
            result_sequence
                .push(cmd_vec)
                .map_err(|_| ExplorerError::BufferOverflow)?;

            for &v in adj_list_rev[u].iter() {
                if !failed_nodes[v] {
                    in_degree[v] -= 1;
                    if in_degree[v] == 0 {
                        q.push(v).map_err(|_| ExplorerError::BufferOverflow)?;
                    }
                }
            }
        }

        if visited_count != len - failed_nodes.iter().filter(|&&f| f).count() {
            return Err(ExplorerError::DependencyCycle);
        }

        Ok((result_sequence, result_len_per_node))
    }
}

pub struct PermutationIter<'a, const N: usize> {
    sequence: &'a [CmdNode],
    total_nodes: usize,
    current_permutation: Vec<&'a [u8], N>,
    used: Vec<bool, N>,
    in_degree: Vec<usize, N>,
    adj_list_rev: Vec<Vec<usize, N>, N>,
    path_stack: Vec<usize, N>, // Stores original indices of commands in current_permutation
    loop_start_indices: Vec<usize, N>, // Tracks the starting point for the next search at each level
    is_done: bool,
}

impl<'a, const N: usize> PermutationIter<'a, N> {
    pub fn new(explorer: &'a Explorer<'a, N>) -> Result<Self, ExplorerError> {
        let total_nodes = explorer.sequence.len();
        if total_nodes > N {
            return Err(ExplorerError::TooManyCommands);
        }

        let mut in_degree: Vec<usize, N> = Vec::new();
        let mut adj_list_rev: Vec<Vec<usize, N>, N> = Vec::new();

        in_degree
            .resize(total_nodes, 0)
            .map_err(|_| ExplorerError::BufferOverflow)?;
        adj_list_rev
            .resize(total_nodes, Vec::new())
            .map_err(|_| ExplorerError::BufferOverflow)?;

        for (i, node) in explorer.sequence.iter().enumerate() {
            in_degree[i] = node.deps.len();
            for &dep in node.deps.iter() {
                if dep >= total_nodes {
                    return Err(ExplorerError::InvalidDependencyIndex);
                }
                adj_list_rev[dep]
                    .push(i)
                    .map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        // Cycle detection using Kahn's algorithm
        let mut temp_in_degree = in_degree.clone();
        let mut q = Vec::<usize, N>::new();
        for i in 0..total_nodes {
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

        if count != total_nodes {
            return Err(ExplorerError::DependencyCycle);
        }

        Ok(Self {
            sequence: explorer.sequence,
            total_nodes,
            current_permutation: Vec::new(),
            used: {
                let mut v = Vec::new();
                v.resize(total_nodes, false)
                    .map_err(|_| ExplorerError::BufferOverflow)?;
                v
            },
            in_degree,
            adj_list_rev,
            path_stack: Vec::new(),
            loop_start_indices: Vec::new(),
            is_done: false,
        })
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
