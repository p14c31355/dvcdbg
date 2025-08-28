// explorer.rs

use crate::scanner::{I2C_SCAN_ADDR_END, I2C_SCAN_ADDR_START};
use heapless::Vec;
use heapless::index_map::FnvIndexMap;
const I2C_ADDRESS_COUNT: usize = 128;

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
        S: core::fmt::Write + crate::logger::Logger<BUF_CAP>;
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
    fn kahn_topo_sort(&self, visited: &Vec<bool, N>) -> Result<Vec<usize, N>, ExplorerError> {
        let mut in_degree: Vec<usize, N> = Vec::new();
        let mut adj_rev: Vec<Vec<usize, N>, N> = Vec::new();
        in_degree
            .resize(self.sequence.len(), 0)
            .map_err(|_| ExplorerError::BufferOverflow)?;
        adj_rev
            .resize(self.sequence.len(), Vec::new())
            .map_err(|_| ExplorerError::BufferOverflow)?;

        for (i, node) in self.sequence.iter().enumerate() {
            in_degree[i] = node.deps.len();
            for &dep in node.deps.iter() {
                if dep >= self.sequence.len() {
                    return Err(ExplorerError::InvalidDependencyIndex);
                }
                adj_rev[dep]
                    .push(i)
                    .map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        let mut q: Vec<usize, N> = Vec::new();
        for i in 0..self.sequence.len() {
            if in_degree[i] == 0 && !visited[i] {
                q.push(i).map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        let mut order: Vec<usize, N> = Vec::new();
        let mut idx = 0;
        while idx < q.len() {
            let u = q[idx];
            idx += 1;
            order.push(u).map_err(|_| ExplorerError::BufferOverflow)?;
            for &v in adj_rev[u].iter() {
                in_degree[v] -= 1;
                if in_degree[v] == 0 && !visited[v] {
                    q.push(v).map_err(|_| ExplorerError::BufferOverflow)?;
                }
            }
        }

        if order.len() != self.sequence.len() - visited.iter().filter(|&&b| b).count() {
            return Err(ExplorerError::DependencyCycle);
        }

        Ok(order)
    }

    pub fn explore<I2C, E, L, const BUF_CAP: usize>(
        &self,
        i2c: &mut I2C,
        executor: &mut E,
        logger: &mut L,
    ) -> Result<ExploreResult, ExplorerError>
    where
        I2C: crate::compat::I2cCompat,
        E: CmdExecutor<I2C, BUF_CAP>,
        L: crate::logger::Logger<BUF_CAP> + core::fmt::Write,
    {
        if self.sequence.is_empty() {
            logger.log_info("[explorer] No commands provided.");
            return Err(ExplorerError::NoValidAddressesFound);
        }

        let mut found_addresses: Vec<u8, I2C_ADDRESS_COUNT> = Vec::new();
        let mut solved_addrs: [bool; I2C_ADDRESS_COUNT] = [false; I2C_ADDRESS_COUNT];
        let mut permutation_count = 0;
        let mut visited_nodes: Vec<bool, N> = Vec::new();
        visited_nodes
            .resize(self.sequence.len(), false)
            .map_err(|_| ExplorerError::BufferOverflow)?;
        let mut hash_table: FnvIndexMap<u64, (), N> = FnvIndexMap::new();
        let mut permutation_iter = PermutationIter::new(self)?;
        let mut failed_sequences_hashes: Vec<u64, N> = Vec::new();
        loop {
            let order = self.kahn_topo_sort(&visited_nodes)?;
            if order.is_empty() {
                break;
            }

            let mut hasher = crc32fast::Hasher::new();
            for &idx in order.iter() {
                hasher.update(self.sequence[idx].bytes);
            }
            let hash = hasher.finalize() as u64;
            if hash_table.contains_key(&hash) {
                for &idx in order.iter() {
                    visited_nodes[idx] = true;
                }
                continue;
            }
            hash_table
                .insert(hash, ())
                .map_err(|_| ExplorerError::BufferOverflow)?;
            permutation_count += 1;

            if failed_sequences_hashes.contains(&current_sequence_hash) {
                logger.log_info_fmt(|buf| {
                    use core::fmt::Write;
                    let _ = write!(
                        buf,
                        "[explorer] Skipping previously failed sequence (hash: 0x{:X})",
                        current_sequence_hash
                    );
                    Ok(())
                });
                continue; // Skip this sequence
            }

            for addr_val in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                let addr_idx = addr_val as usize;
                if solved_addrs[addr_idx] {
                    continue;
                }

                let mut all_ok = true;
                for addr_val in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                    for &cmd_bytes in sequence_bytes.iter() {
                        if let Err(e) = executor.exec(i2c, addr_val, cmd_bytes, logger) {
                            all_ok = false;
                            logger.log_error_fmt(|buf| {
                                use core::fmt::Write;
                                let _ = write!(
                                    buf,
                                    "[explorer] Execution failed for addr 0x{:02X}: {:?}\r\n",
                                    addr_val, e
                                );
                                Ok(())
                            });
                            break;
                        }
                    }
                }

                if all_ok {
                    solved_addrs[addr_idx] = true;
                    found_addresses
                        .push(addr_val)
                        .map_err(|_| ExplorerError::BufferOverflow)?;
                }

                if !all_ok {
                    failed_sequences_hashes
                        .insert(current_sequence_hash, ())
                        .map_err(|_| ExplorerError::BufferOverflow)?;
                }
            }

            for &idx in order.iter() {
                visited_nodes[idx] = true;
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
        serial: &mut impl core::fmt::Write,
    ) -> Result<([[u8; MAX_CMD_LEN]; N], [usize; N]), ExplorerError> {
        let len = self.sequence.len();

        let mut in_degree: [usize; N] = [0; N];
        let mut adj_list_rev: [[usize; N]; N] = [[0; N]; N];
        let mut adj_list_len: [usize; N] = [0; N];

        let mut result_sequence: [[u8; MAX_CMD_LEN]; N] = [[0; MAX_CMD_LEN]; N];
        let mut result_len_per_node: [usize; N] = [0; N];
        let mut result_len = 0;

        for (i, node) in self.sequence.iter().enumerate() {
            in_degree[i] = node.deps.len();
            writeln!(
                serial,
                "[dbg] Node {i} deps={:?}, in_degree={}",
                node.deps, in_degree[i]
            )
            .ok();

            for &dep_idx in node.deps.iter() {
                if dep_idx >= len {
                    writeln!(
                        serial,
                        "[error] Node {i} has invalid dep index {dep_idx} (len={len})"
                    )
                    .ok();
                    return Err(ExplorerError::InvalidDependencyIndex);
                }
                let pos = adj_list_len[dep_idx];
                adj_list_rev[dep_idx][pos] = i;
                adj_list_len[dep_idx] += 1;
            }
        }

        let mut q: [usize; N] = [0; N];
        let mut head = 0;
        let mut tail = 0;
        for (i, &degree) in in_degree.iter().enumerate().take(len) {
            if degree == 0 {
                q[tail] = i;
                tail += 1;
            }
        }

        while head < tail {
            let u = q[head];
            head += 1;

            let cmd_bytes = self.sequence[u].bytes;
            let copy_len = cmd_bytes.len().min(MAX_CMD_LEN);
            result_sequence[result_len][..copy_len].copy_from_slice(&cmd_bytes[..copy_len]);
            result_len_per_node[result_len] = copy_len;
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

        for i in 0..len {
            writeln!(
                serial,
                "[dbg] Node {i} bytes={:02X?} (len={})",
                &result_sequence[i][..result_len_per_node[i]],
                result_len_per_node[i]
            )
            .ok();
        }

        Ok((result_sequence, result_len_per_node))
    }
}

/// An iterator that generates all valid topological permutations of the command sequence.
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
