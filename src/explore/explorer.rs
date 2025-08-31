// explorer.rs

use crate::compat::err_compat::HalErrorExt;
use crate::error::{ExecutorError, ExplorerError};
use core::fmt::Write;

use crate::scanner::{I2C_SCAN_ADDR_END, I2C_SCAN_ADDR_START};
use heapless::Vec;
const I2C_ADDRESS_COUNT: usize = 128;

#[derive(Copy, Clone)]
pub struct CmdNode {
    pub bytes: &'static [u8],
    pub deps: &'static [u8],
}

pub trait CmdExecutor<I2C, const CMD_BUFFER_SIZE: usize> {
    // Use CMD_BUFFER_SIZE
    fn exec<L: crate::explore::logger::Logger + core::fmt::Write>(
        &mut self,
        i2c: &mut I2C,
        addr: u8,
        cmd: &[u8],
        logger: &mut L,
    ) -> Result<(), ExecutorError>;
}

/// A command executor that prepends a prefix to each command.
pub struct PrefixExecutor<const INIT_SEQUENCE_LEN: usize, const CMD_BUFFER_SIZE: usize> {
    // Use INIT_SEQUENCE_LEN and CMD_BUFFER_SIZE
    buffer: Vec<u8, CMD_BUFFER_SIZE>, // Use CMD_BUFFER_SIZE
    initialized_addrs: [bool; I2C_ADDRESS_COUNT],
    prefix: u8,
    init_sequence: heapless::Vec<u8, INIT_SEQUENCE_LEN>, // Capacity is the length of the init sequence
}

impl<const INIT_SEQUENCE_LEN: usize, const CMD_BUFFER_SIZE: usize>
    PrefixExecutor<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>
{
    pub fn new(prefix: u8, init_sequence: heapless::Vec<u8, INIT_SEQUENCE_LEN>) -> Self {
        Self {
            buffer: Vec::new(),
            initialized_addrs: [false; I2C_ADDRESS_COUNT],
            prefix,
            init_sequence,
        }
    }

    // Private helper for short delay
    fn short_delay() {
        for _ in 0..1_000 {
            core::hint::spin_loop();
        }
    }

    // Private helper for write with retry
    fn write_with_retry<I2C, S>(
        i2c: &mut I2C,
        addr: u8,
        bytes: &[u8],
        logger: &mut S,
    ) -> Result<(), crate::error::ErrorKind>
    where
        I2C: crate::compat::I2cCompat,
        <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
        S: core::fmt::Write + crate::explore::logger::Logger,
    {
        let mut last_error = None;
        for _attempt in 0..2 {
            match i2c.write(addr, bytes) {
                Ok(_) => {
                    Self::short_delay();
                    return Ok(());
                }
                Err(e) => {
                    let compat_err = e.to_compat(Some(addr));
                    last_error = Some(compat_err);
                    let _ = logger
                        .log_error_fmt(|buf| writeln!(buf, "[I2C retry error] {compat_err:?}"));
                    Self::short_delay();
                }
            }
        }
        Err(last_error.unwrap_or(crate::error::ErrorKind::I2c(crate::error::I2cError::Nack)))
    }
}

pub fn execute_and_log_command<I2C, E, L, const MAX_BYTES_PER_CMD: usize>(
    i2c: &mut I2C,
    executor: &mut E,
    logger: &mut L,
    addr: u8,
    cmd_bytes: &[u8],
    cmd_idx: usize,
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    E: CmdExecutor<I2C, MAX_BYTES_PER_CMD>,
    L: crate::explore::logger::Logger + core::fmt::Write,
{
    let _ = logger.log_info_fmt(|buf| {
        writeln!(
            buf,
            "[explorer] Sending node {} bytes: {:02X?} ...",
            cmd_idx, cmd_bytes
        )
    });

    match executor.exec(i2c, addr, cmd_bytes, logger) {
        Ok(_) => {
            let _ = logger.log_info_fmt(|buf| writeln!(buf, "[explorer] OK"));
            Ok(())
        }
        Err(e) => {
            let _ = logger.log_error_fmt(|buf| writeln!(buf, "[explorer] FAILED: {:?}", e));
            Err(e.into())
        }
    }
}

impl<I2C, const INIT_SEQ_SIZE: usize, const CMD_BUFFER_SIZE: usize>
    CmdExecutor<I2C, CMD_BUFFER_SIZE> for PrefixExecutor<INIT_SEQ_SIZE, CMD_BUFFER_SIZE>
// Use CMD_BUFFER_SIZE
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
        S: core::fmt::Write + crate::explore::logger::Logger,
    {
        let addr_idx = addr as usize;

        if !self.initialized_addrs[addr_idx] && !self.init_sequence.is_empty() {
            let _ = logger
                .log_info_fmt(|buf| writeln!(buf, "[Info] I2C initializing for 0x{addr:02X}..."));
            let ack_ok = Self::write_with_retry(i2c, addr, &[addr], logger).is_ok();
            if ack_ok {
                // self.prefix = addr; // Removed this line as it mutates the fixed prefix
                let _ = logger.log_info_fmt(|buf| {
                    writeln!(
                        buf,
                        "[Info] Device found at 0x{addr:02X}, sending init sequence..."
                    )
                });
                for &c in self.init_sequence.iter() {
                    let command = [self.prefix, c]; // Uses the original self.prefix
                    Self::write_with_retry(i2c, addr, &command, logger)
                        .map_err(ExecutorError::I2cError)?;
                    Self::short_delay();
                }
                self.initialized_addrs[addr_idx] = true;
                let _ = logger
                    .log_info_fmt(|buf| writeln!(buf, "[Info] I2C initialized for 0x{addr:02X}"));
            }
        }
        let prefix = self.prefix; // Changed to use the instance's prefix
        self.buffer.clear();
        self.buffer
            .push(prefix)
            .map_err(|_| ExecutorError::BufferOverflow)?;
        self.buffer
            .extend_from_slice(cmd)
            .map_err(|_| ExecutorError::BufferOverflow)?;
        Self::write_with_retry(i2c, addr, &self.buffer, logger).map_err(ExecutorError::I2cError)
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
    // This function calculates the max length of a single command's byte array
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
        max_len
    }

    pub fn explore<I2C, E, L, const CMD_BUFFER_SIZE: usize>(
        // Use CMD_BUFFER_SIZE
        &self,
        i2c: &mut I2C,
        executor: &mut E,
        logger: &mut L,
    ) -> Result<ExploreResult, ExplorerError>
    where
        I2C: crate::compat::I2cCompat,
        <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
        E: CmdExecutor<I2C, CMD_BUFFER_SIZE>, // Use CMD_BUFFER_SIZE
        L: crate::explore::logger::Logger + core::fmt::Write,
    {
        if self.sequence.is_empty() {
            let _ = logger.log_info_fmt(|buf| writeln!(buf, "[explorer] No commands provided."));
            return Err(ExplorerError::NoValidAddressesFound);
        }

        let mut found_addrs: Vec<u8, I2C_ADDRESS_COUNT> = Vec::new();
        let mut solved_addrs: [bool; I2C_ADDRESS_COUNT] = [false; I2C_ADDRESS_COUNT];
        let mut permutations_tested = 0;
        let iter = PermutationIter::new(self)?;
        let _ = logger
            .log_info_fmt(|buf| writeln!(buf, "[explorer] Starting permutation exploration..."));
        for sequence in iter {
            permutations_tested += 1;
            for addr_val in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                let addr = addr_val;
                if solved_addrs[addr as usize] {
                    continue;
                }

                let _ = logger.log_info_fmt(|buf| {
                    writeln!(
                        buf,
                        "[explorer] Trying sequence on 0x{:02X} (permutation {})",
                        addr, permutations_tested
                    )
                });

                let mut all_ok = true;
                for i in 0..self.sequence.len() {
                    let cmd_bytes = sequence[i];
                    if execute_and_log_command(i2c, executor, logger, addr, cmd_bytes, i).is_err() {
                        all_ok = false;
                        break;
                    }
                }

                if all_ok {
                    let _ = logger.log_info_fmt(|buf| {
                        writeln!(
                            buf,
                            "[explorer] Successfully executed sequence on 0x{:02X}",
                            addr
                        )
                    });
                    if found_addrs.push(addr).is_err() {
                        let _ = logger.log_error_fmt(|buf| {
                            writeln!(buf, "[error] Buffer overflow in found_addrs")
                        });
                        return Err(ExplorerError::BufferOverflow);
                    }
                    solved_addrs[addr as usize] = true;
                } else {
                    let _ = logger.log_info_fmt(|buf| {
                        writeln!(
                            buf,
                            "[explorer] Failed to execute sequence on 0x{:02X}",
                            addr
                        )
                    });
                }
            }

            if found_addrs.len() == (I2C_SCAN_ADDR_END - I2C_SCAN_ADDR_START + 1) as usize {
                break;
            }
        }
        Ok(ExploreResult {
            found_addrs,
            permutations_tested,
        })
    }

    pub fn get_one_topological_sort_buf(
        &self,
        _serial: &mut impl core::fmt::Write,
        failed_nodes: &[bool; N],
    ) -> Result<(heapless::Vec<&'a [u8], N>, heapless::Vec<u8, N>), ExplorerError> {
        let len = self.sequence.len();
        let mut in_degree: heapless::Vec<u8, N> = heapless::Vec::new();
        in_degree
            .resize(len, 0)
            .map_err(|_| ExplorerError::BufferOverflow)?;
        let mut adj_list_rev: heapless::Vec<heapless::Vec<u8, N>, N> = heapless::Vec::new();
        adj_list_rev
            .resize(len, heapless::Vec::new())
            .map_err(|_| ExplorerError::BufferOverflow)?;

        for (i, node) in self.sequence.iter().enumerate() {
            if failed_nodes[i] {
                continue;
            }
            in_degree[i] = node.deps.len() as u8; // node.deps.len() is usize, cast to u8
            for &dep_idx in node.deps.iter() {
                let dep_idx_usize = dep_idx as usize;
                if dep_idx_usize >= len {
                    return Err(ExplorerError::InvalidDependencyIndex);
                }
                // dep_idx is u8, i is usize. adj_list_rev expects u8 for its inner Vec.
                adj_list_rev[dep_idx_usize]
                    .push(i as u8)
                    .map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        let mut result_sequence: heapless::Vec<&'a [u8], N> = heapless::Vec::new(); // Changed type
        let mut result_len_per_node: heapless::Vec<u8, N> = heapless::Vec::new(); // Changed type to u8
        let mut visited_count = 0;

        let mut q = heapless::Vec::<u8, N>::new(); // Initialize the queue 'q'
        for i in 0..len {
            if in_degree[i] == 0 && !failed_nodes[i] {
                q.push(i as u8).map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        while let Some(u_u8) = q.pop() {
            // u_u8 is u8
            let u = u_u8 as usize; // Cast to usize for indexing
            visited_count += 1;

            let cmd_bytes = self.sequence[u].bytes;
            result_len_per_node
                .push(cmd_bytes.len() as u8) // Push u8
                .map_err(|_| ExplorerError::BufferOverflow)?;
            result_sequence
                .push(cmd_bytes)
                .map_err(|_| ExplorerError::BufferOverflow)?;

            for &v_u8 in adj_list_rev[u].iter() {
                // v_u8 is u8
                let v = v_u8 as usize; // Cast to usize for indexing
                if !failed_nodes[v] {
                    in_degree[v] -= 1;
                    if in_degree[v] == 0 {
                        q.push(v_u8).map_err(|_| ExplorerError::BufferOverflow)?; // Push u8
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
    pub explorer: &'a Explorer<'a, N>,
    pub total_nodes: usize,
    pub current_permutation: Vec<&'a [u8], N>,
    // pub used: Vec<bool, N>, // REMOVE THIS LINE
    pub used: Vec<bool, N>,
    pub in_degree: Vec<u8, N>,
    pub adj_list_rev: Vec<heapless::Vec<u8, N>, N>,
    pub path_stack: Vec<u8, N>,
    pub loop_start_indices: Vec<u8, N>,
    pub is_done: bool,
}

impl<'a, const N: usize> PermutationIter<'a, N> {
    pub fn new(explorer: &'a Explorer<'a, N>) -> Result<Self, ExplorerError> {
        let total_nodes = explorer.sequence.len();
        if total_nodes > N {
            return Err(ExplorerError::TooManyCommands);
        }

        let mut in_degree: Vec<u8, N> = Vec::new();
        let mut adj_list_rev: Vec<heapless::Vec<u8, N>, N> = Vec::new();
        adj_list_rev
            .resize(total_nodes, heapless::Vec::new())
            .map_err(|_| ExplorerError::BufferOverflow)?;

        in_degree
            .resize(total_nodes, 0)
            .map_err(|_| ExplorerError::BufferOverflow)?;
        for (i, node) in explorer.sequence.iter().enumerate() {
            in_degree[i as usize] = node.deps.len() as u8; // Cast i to usize
            for &dep in node.deps.iter() {
                if dep as usize >= total_nodes {
                    // Cast dep to usize for comparison
                    return Err(ExplorerError::InvalidDependencyIndex);
                }
                adj_list_rev[dep as usize]
                    .push(i as u8)
                    .map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        // Cycle detection using Kahn's algorithm
        let mut temp_in_degree = in_degree.clone();
        let mut q = Vec::<u8, N>::new();
        for i in 0..total_nodes {
            if temp_in_degree[i] == 0 {
                q.push(i as u8).map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        let mut count = 0;
        let mut q_idx = 0;
        while q_idx < q.len() {
            let u = q[q_idx] as usize;
            q_idx += 1;
            count += 1;

            for &v_u8 in adj_list_rev[u].iter() {
                let v = v_u8 as usize;
                temp_in_degree[v] -= 1;
                if temp_in_degree[v] == 0 {
                    q.push(v_u8).map_err(|_| ExplorerError::BufferOverflow)?;
                }
            }
        }

        if count != total_nodes {
            return Err(ExplorerError::DependencyCycle);
        }

        Ok(Self {
            explorer,
            total_nodes,
            current_permutation: Vec::new(),
            // used: Vec::new(), // REMOVE THIS LINE
            used_mask: 0, // ADD THIS LINE: Initialize the bitmask to 0
            in_degree,
            adj_list_rev,
            path_stack: Vec::new(),
            loop_start_indices: Vec::new(),
            is_done: false,
        })
    }

    fn try_extend(&mut self) -> bool {
        let current_depth = self.current_permutation.len();
        // The `loop_start_indices` tracks the starting point for the search at the current depth.
        // If it's empty, we start from the beginning (0). Otherwise, we continue from where we left off.
        let start_idx = self
            .loop_start_indices
            .get(current_depth)
            .copied()
            .unwrap_or(0) as usize;

        // Iterate through all possible nodes (0 to total_nodes-1)
        for i in start_idx..self.total_nodes {
            // Check if node 'i' is NOT used (bit is 0) AND its in-degree is 0
            if ((self.used_mask >> i) & 1) == 0 && self.in_degree[i] == 0 {
                // Mark node 'i' as used (set its bit)
                self.used_mask |= 1 << i;
                self.current_permutation
                    .push(self.explorer.sequence[i].bytes)
                    .ok();
                self.path_stack.push(i as u8).ok();
                self.loop_start_indices.push((i + 1) as u8).ok();
                for &neighbor_u8 in self.adj_list_rev[i].iter() {
                    let neighbor = neighbor_u8 as usize;
                    self.in_degree[neighbor] -= 1;
                }
                return true;
            }
        }
        false
    }

    fn backtrack(&mut self) -> bool {
        if let Some(last_added_idx_u8) = self.path_stack.pop() {
            let last_added_idx = last_added_idx_u8 as usize;
            self.current_permutation.pop();
            // Unmark node 'last_added_idx' as used (clear its bit)
            self.used_mask &= !(1 << last_added_idx);
            self.loop_start_indices.pop();

            for &neighbor_u8 in self.adj_list_rev[last_added_idx].iter() {
                let neighbor = neighbor_u8 as usize;
                self.in_degree[neighbor] += 1;
            }

            // If path_stack is empty after pop, we've backtracked past the root
            if self.path_stack.is_empty() {
                // Removed current_permutation.is_empty() check
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

impl<'a, const N: usize> Iterator for PermutationIter<'a, N> {
    type Item = Vec<&'a [u8], N>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }

        loop {
            // If we have a complete permutation, return it and prepare for the next one.
            if self.current_permutation.len() == self.total_nodes {
                // Optimize SRAM: Use core::mem::take to move the Vec out, avoiding a clone.
                // This leaves an empty Vec in its place, which will be refilled by subsequent
                // calls to try_extend or backtrack.
                let full_sequence = core::mem::take(&mut self.current_permutation);

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
