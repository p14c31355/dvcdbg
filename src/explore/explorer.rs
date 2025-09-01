// explorer.rs

use crate::compat::err_compat::HalErrorExt;
use crate::compat::util;
use crate::error::{ExecutorError, ExplorerError};

use crate::scanner::{I2C_SCAN_ADDR_END, I2C_SCAN_ADDR_START};
const I2C_ADDRESS_COUNT: usize = 128;

#[derive(Copy, Clone)]
pub struct CmdNode {
    pub bytes: &'static [u8],
    pub deps: &'static [u8],
}

pub trait CmdExecutor<I2C, const CMD_BUFFER_SIZE: usize> {
    // Use CMD_BUFFER_SIZE
    fn exec<W: core::fmt::Write>(
        &mut self,
        i2c: &mut I2C,
        addr: u8,
        cmd: &[u8],
        writer: &mut W,
    ) -> Result<(), ExecutorError>;
}

/// A command executor that prepends a prefix to each command.
pub struct PrefixExecutor<const INIT_SEQUENCE_LEN: usize, const CMD_BUFFER_SIZE: usize> {
    buffer: [u8; CMD_BUFFER_SIZE],
    buffer_len: usize,
    initialized_addrs: util::BitFlags, // No generic parameter needed now
    prefix: u8,
    init_sequence: [u8; INIT_SEQUENCE_LEN],
    init_sequence_len: usize,
}

impl<const INIT_SEQUENCE_LEN: usize, const CMD_BUFFER_SIZE: usize>
    PrefixExecutor<INIT_SEQUENCE_LEN, CMD_BUFFER_SIZE>
{
    pub fn new(prefix: u8, init_sequence: &[u8]) -> Self {
        let mut init_seq_arr = [0u8; INIT_SEQUENCE_LEN];
        let init_seq_len = init_sequence.len().min(INIT_SEQUENCE_LEN);
        if init_seq_len > 0 {
            init_seq_arr[..init_seq_len].copy_from_slice(&init_sequence[..init_seq_len]);
        }

        Self {
            buffer: [0; CMD_BUFFER_SIZE],
            buffer_len: 0,
            initialized_addrs: util::BitFlags::new(), // No generic parameter
            prefix,
            init_sequence: init_seq_arr,
            init_sequence_len: init_seq_len,
        }
    }

    // Private helper for short delay
    fn short_delay() {
        for _ in 0..1_000 {
            core::hint::spin_loop();
        }
    }

    // Private helper for write with retry
    fn write_with_retry<I2C, W>(
        i2c: &mut I2C,
        addr: u8,
        bytes: &[u8],
        writer: &mut W,
    ) -> Result<(), crate::error::ErrorKind>
    where
        I2C: crate::compat::I2cCompat,
        <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
        W: core::fmt::Write,
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
                    writeln!(writer, "[I2C retry error] {compat_err}").ok();
                    Self::short_delay();
                }
            }
        }
        Err(last_error.unwrap_or(crate::error::ErrorKind::I2c(crate::error::I2cError::Nack)))
    }
}

pub fn exec_log_cmd<I2C, E, W, const MAX_BYTES_PER_CMD: usize>(
    i2c: &mut I2C,
    executor: &mut E,
    writer: &mut W,
    addr: u8,
    cmd_bytes: &[u8],
    cmd_idx: usize,
) -> Result<(), ExplorerError>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
    E: CmdExecutor<I2C, MAX_BYTES_PER_CMD>,
    W: core::fmt::Write,
{
    // Replaced writeln! with prevent_garbled
    util::prevent_garbled(
        writer,
        format_args!("[explorer] Sending node {cmd_idx} bytes: {cmd_bytes:02X?} ..."),
    );

    match executor.exec(i2c, addr, cmd_bytes, writer) {
        Ok(_) => {
            // Replaced writeln! with prevent_garbled
            util::prevent_garbled(writer, format_args!("[explorer] OK"));
            Ok(())
        }
        Err(e) => {
            // Replaced writeln! with prevent_garbled
            util::prevent_garbled(writer, format_args!("[explorer] FAILED: {e}"));
            Err(e.into())
        }
    }
}

impl<I2C, const INIT_SEQ_SIZE: usize, const CMD_BUFFER_SIZE: usize>
    CmdExecutor<I2C, CMD_BUFFER_SIZE> for PrefixExecutor<INIT_SEQ_SIZE, CMD_BUFFER_SIZE>
where
    I2C: crate::compat::I2cCompat,
    <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
{
    fn exec<W>(
        &mut self,
        i2c: &mut I2C,
        addr: u8,
        cmd: &[u8],
        writer: &mut W,
    ) -> Result<(), ExecutorError>
    where
        W: core::fmt::Write,
    {
        let addr_idx = addr as usize;

        if !self
            .initialized_addrs
            .get(addr_idx)
            .map_err(ExecutorError::BitFlagsError)?
            && self.init_sequence_len > 0
        {
            // Check for buffer space for batched init sequence
            if (self.init_sequence_len * 2) > CMD_BUFFER_SIZE {
                return Err(ExecutorError::BufferOverflow);
            }

            write!(writer, "[Info] I2C initializing for ").ok();
            util::write_bytes_hex_fmt(writer, &[addr]).ok();
            writeln!(writer, "...").ok();
            let ack_ok = Self::write_with_retry(i2c, addr, &[], writer).is_ok();

            if ack_ok {
                write!(writer, "[Info] Device found at ").ok();
                util::write_bytes_hex_fmt(writer, &[addr]).ok();
                writeln!(writer, ", sending init sequence...").ok();

                // Batch the init sequence
                for (i, &c) in self.init_sequence[..self.init_sequence_len]
                    .iter()
                    .enumerate()
                {
                    self.buffer[2 * i] = self.prefix;
                    self.buffer[2 * i + 1] = c;
                }

                Self::write_with_retry(
                    i2c,
                    addr,
                    &self.buffer[..self.init_sequence_len * 2],
                    writer,
                )
                .map_err(ExecutorError::I2cError)?;

                Self::short_delay();

                self.initialized_addrs
                    .set(addr_idx)
                    .map_err(ExecutorError::BitFlagsError)?;
                write!(writer, "[Info] I2C initialized for ").ok();
                util::write_bytes_hex_fmt(writer, &[addr]).ok();
                writeln!(writer).ok();
            }
        }

        // Existing command execution logic remains similar
        self.buffer_len = 0;
        self.buffer[self.buffer_len] = self.prefix;
        self.buffer_len += 1;

        if self.buffer_len + cmd.len() > CMD_BUFFER_SIZE {
            return Err(ExecutorError::BufferOverflow);
        }
        let end = self.buffer_len + cmd.len();
        self.buffer[self.buffer_len..end].copy_from_slice(cmd);
        self.buffer_len = end;

        Self::write_with_retry(i2c, addr, &self.buffer[..self.buffer_len], writer)
            .map_err(ExecutorError::I2cError)
    }
}

#[macro_export]
macro_rules! nodes {
    (
        prefix = $prefix:expr,
        [ $( [ $( $b:expr ),* ] $( @ [ $( $d:expr ),* ] )? ),* $(,)? ]
    ) => {{
        const NODES: [$crate::CmdNode; count_exprs!($( [ $( $b ),* ] ),*)] = [
            $(
                $crate::CmdNode {
                    bytes: &[ $( $b ),* ],
                    deps: &[ $( $( $d ),* )? ],
                }
            ),*
        ];

        const EXPLORER: $crate::Explorer<{NODES.len()}> = $crate::Explorer::new(&NODES);

        const CMD_BUFFER_SIZE: usize = 1 + {
            let mut max_len = 0;
            let mut i = 0;
            while i < NODES.len() {
                let len = NODES[i].bytes.len();
                if len > max_len {
                    max_len = len;
                }
                i += 1;
            }
            max_len
        };

        (
            &EXPLORER,
            $crate::PrefixExecutor::<0, CMD_BUFFER_SIZE>::new($prefix, &[])
        )
    }};
}

/// simple macro to count comma-separated expressions at compile time
#[macro_export]
macro_rules! count_exprs {
    () => (0usize);
    ($x:expr $(, $xs:expr)*) => (1usize + $crate::count_exprs!($($xs),*));
}

pub struct Explorer<const N: usize> {
    pub(crate) nodes: &'static [CmdNode],
}

pub struct ExploreResult {
    pub found_addrs: [u8; I2C_ADDRESS_COUNT],
    pub found_addrs_len: usize,
    pub permutations_tested: usize,
}

impl<const N: usize> Explorer<N> {
    // This function calculates the max length of a single command's byte array
    pub const fn max_cmd_len(&self) -> usize {
        let mut max_len = 0;
        let mut i = 0;
        while i < N {
            let len = self.nodes[i].bytes.len();
            if len > max_len {
                max_len = len;
            }
            i += 1;
        }
        max_len
    }

    pub const fn new(nodes: &'static [CmdNode]) -> Self {
        Self { nodes }
    }

    pub fn explore<I2C, E, W, const CMD_BUFFER_SIZE: usize>(
        // Use CMD_BUFFER_SIZE
        &self,
        i2c: &mut I2C,
        executor: &mut E,
        writer: &mut W,
    ) -> Result<ExploreResult, ExplorerError>
    where
        I2C: crate::compat::I2cCompat,
        <I2C as crate::compat::I2cCompat>::Error: crate::compat::HalErrorExt,
        E: CmdExecutor<I2C, CMD_BUFFER_SIZE>,
        W: core::fmt::Write,
    {
        if self.nodes.is_empty() {
            writeln!(writer, "[explorer] No commands provided.").ok();
            return Err(ExplorerError::NoValidAddressesFound);
        }

        let mut found_addrs: [u8; I2C_ADDRESS_COUNT] = [0; I2C_ADDRESS_COUNT];
        let mut found_addrs_len: usize = 0;
        let mut solved_addrs: util::BitFlags = util::BitFlags::new();
        let mut permutations_tested = 0;
        let iter = PermutationIter::new(self)?;
        writeln!(writer, "[explorer] Starting permutation exploration...").ok();
        for sequence in iter {
            permutations_tested += 1;

            for addr_val in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                let addr = addr_val;
                if solved_addrs
                    .get(addr as usize)
                    .map_err(ExplorerError::BitFlagsError)?
                {
                    continue;
                }

                write!(writer, "[explorer] Trying sequence on ").ok();
                util::write_bytes_hex_fmt(writer, &[addr]).ok();
                writeln!(writer, " (permutation {permutations_tested})").ok();

                let mut all_ok = true;

                for (i, &cmd_bytes) in sequence.iter().enumerate().take(self.nodes.len()) {
                    if exec_log_cmd(i2c, executor, writer, addr, cmd_bytes, i).is_err() {
                        all_ok = false;
                        break;
                    }
                }

                if all_ok {
                    write!(writer, "[explorer] Successfully executed sequence on ").ok();
                    util::write_bytes_hex_fmt(writer, &[addr]).ok();
                    writeln!(writer).ok();

                    if found_addrs_len < I2C_ADDRESS_COUNT {
                        found_addrs[found_addrs_len] = addr;
                        found_addrs_len += 1;
                    } else {
                        writeln!(writer, "[error] Buffer overflow in found_addrs").ok();
                        return Err(ExplorerError::BufferOverflow);
                    }
                    solved_addrs
                        .set(addr as usize)
                        .map_err(ExplorerError::BitFlagsError)?;
                } else {
                    write!(writer, "[explorer] Failed to execute sequence on ").ok();
                    util::write_bytes_hex_fmt(writer, &[addr]).ok();
                    writeln!(writer).ok();
                }
            }

            if found_addrs_len == (I2C_SCAN_ADDR_END - I2C_SCAN_ADDR_START + 1) as usize {
                break;
            }
        }
        Ok(ExploreResult {
            found_addrs,
            found_addrs_len,
            permutations_tested,
        })
    }

    pub fn get_one_sort(
        &self,
        _writer: &mut impl core::fmt::Write,
        failed_nodes: &[bool; N],
    ) -> Result<(heapless::Vec<&'static [u8], N>, heapless::Vec<u8, N>), ExplorerError> {
        let len = self.nodes.len();
        let mut in_degree: [u8; N] = [0; N];
        let mut adj_list_rev: [u128; N] = [0; N];

        // Ensure N is large enough for the sequence
        if len > N {
            return Err(ExplorerError::TooManyCommands);
        }

        // Build the graph representation using fixed-size arrays
        for (i, node) in self.nodes.iter().enumerate().take(len) {
            if failed_nodes[i] {
                continue;
            }
            in_degree[i] = node.deps.len() as u8;
            for &dep_idx in node.deps.iter() {
                let dep_idx_usize = dep_idx as usize;
                if dep_idx_usize >= len {
                    return Err(ExplorerError::InvalidDependencyIndex);
                }
                // Use a bitmask (u128) to represent the adjacency list.
                // This replaces the heapless::Vec<heapless::Vec<u8, N>, N> from the original.
                adj_list_rev[dep_idx_usize] |= 1 << i;
            }
        }

        let mut result_sequence: heapless::Vec<&[u8], N> = heapless::Vec::new();
        let mut result_len_per_node: heapless::Vec<u8, N> = heapless::Vec::new();
        let mut visited_count = 0;

        // Use a fixed-size array as a queue to avoid heap allocation.
        // `q_head` and `q_tail` manage the queue's state.
        let mut q: [u8; N] = [0; N];
        let mut q_head: usize = 0;
        let mut q_tail: usize = 0;

        // Initialize the queue with nodes that have an in-degree of 0.
        for i in 0..len {
            if in_degree[i] == 0 && !failed_nodes[i] {
                if q_tail >= N {
                    return Err(ExplorerError::BufferOverflow);
                }
                q[q_tail] = i as u8;
                q_tail += 1;
            }
        }

        // Main topological sort loop
        while q_head < q_tail {
            let u_u8 = q[q_head];
            q_head += 1;
            let u = u_u8 as usize;
            visited_count += 1;

            let cmd_bytes = self.nodes[u].bytes;
            result_len_per_node
                .push(cmd_bytes.len() as u8)
                .map_err(|_| ExplorerError::BufferOverflow)?;
            result_sequence
                .push(cmd_bytes)
                .map_err(|_| ExplorerError::BufferOverflow)?;

            // Iterate through neighbors of 'u' using the bitmask
            for v in 0..len {
                if (adj_list_rev[u] >> v) & 1 != 0 && !failed_nodes[v] {
                    in_degree[v] -= 1;
                    if in_degree[v] == 0 {
                        if q_tail >= N {
                            return Err(ExplorerError::BufferOverflow);
                        }
                        q[q_tail] = v as u8;
                        q_tail += 1;
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
    pub explorer: &'a Explorer<N>,
    pub total_nodes: usize,
    pub current_permutation: [&'a [u8]; N],
    pub current_permutation_len: u8,
    pub used: util::BitFlags, // No generic parameter needed now
    pub in_degree: [u8; N],
    pub adj_list_rev: [u128; N],
    pub path_stack: [u8; N],
    pub path_stack_len: u8,
    pub is_done: bool,
}

impl<'a, const N: usize> PermutationIter<'a, N> {
    pub fn new(explorer: &'a Explorer<N>) -> Result<Self, ExplorerError> {
        const {
            assert!(
                N <= 128,
                "PermutationIter uses a u128 bitmask, so N cannot exceed 128"
            );
        };

        let total_nodes = explorer.nodes.len();
        if total_nodes > N {
            return Err(ExplorerError::TooManyCommands);
        }

        let mut in_degree: [u8; N] = [0; N];
        let mut adj_list_rev: [u128; N] = [0; N];

        for (i, node) in explorer.nodes.iter().enumerate() {
            in_degree[i] = node.deps.len() as u8;
            for &dep in node.deps.iter() {
                if dep as usize >= total_nodes {
                    return Err(ExplorerError::InvalidDependencyIndex);
                }
                adj_list_rev[dep as usize] |= 1 << (i as u128);
            }
        }

        // Cycle detection using Kahn's algorithm
        let mut temp_in_degree = in_degree;
        let mut q: heapless::Vec<u8, N> = heapless::Vec::new();
        let mut q_head: usize = 0;
        let mut q_tail: usize = 0;
        for (i, _) in temp_in_degree.iter().enumerate().take(total_nodes) {
            if temp_in_degree[i] == 0 {
                if q_tail >= N {
                    return Err(ExplorerError::BufferOverflow);
                }
                q[q_tail] = i as u8;
                q_tail += 1;
            }
        }

        let mut _count = 0;
        while q_head < q_tail {
            let u = q[q_head] as usize;
            q_head += 1;
            _count += 1;

            for v in 0..total_nodes {
                if (adj_list_rev[u] >> v) & 1 != 0 {
                    temp_in_degree[v] -= 1;
                    if temp_in_degree[v] == 0 {
                        if q_tail >= N {
                            return Err(ExplorerError::BufferOverflow);
                        }
                        q[q_tail] = v as u8;
                        q_tail += 1;
                    }
                }
            }
        }
        for (i, _) in temp_in_degree.iter().enumerate().take(total_nodes) {
            if temp_in_degree[i] == 0 {
                q.push(i as u8).map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        let mut _count = 0;
        let mut q_idx = 0;
        while q_idx < q.len() {
            let u = q[q_idx] as usize;
            q_idx += 1;
            _count += 1;

            for v in 0..total_nodes {
                if (adj_list_rev[u] >> v) & 1 != 0 {
                    temp_in_degree[v] -= 1;
                }
            }
        }

        if _count != total_nodes {
            return Err(ExplorerError::DependencyCycle);
        }

        Ok(Self {
            explorer,
            total_nodes,
            current_permutation: [b""; N], // Corrected: Initialize with N empty byte slices
            current_permutation_len: 0,
            used: util::BitFlags::new(), // Corrected: No generic parameter
            in_degree,
            adj_list_rev,
            path_stack: [0; N],
            path_stack_len: 0,
            is_done: false,
            // Removed q, q_head, q_tail from here as they are local variables
        })
    }

    fn try_extend(&mut self) -> bool {
        let _current_depth = self.current_permutation_len as usize;

        for i in 0..self.total_nodes {
            let used = match self.used.get(i) {
                Ok(u) => u,
                Err(_) => {
                    // This should not happen given the bounds checks, but handle gracefully.
                    self.is_done = true;
                    return false;
                }
            };
            if !used && self.in_degree[i] == 0 {
                // Mark node 'i' as used
                self.used.set(i).unwrap();
                if self.current_permutation_len < N as u8 {
                    self.current_permutation[self.current_permutation_len as usize] =
                        self.explorer.nodes[i].bytes;
                    self.current_permutation_len += 1;
                } else {
                    self.is_done = true;
                }
                // Push to path_stack
                if self.path_stack_len < N as u8 {
                    self.path_stack[self.path_stack_len as usize] = i as u8;
                    self.path_stack_len += 1;
                } else {
                    self.is_done = true;
                }

                for neighbor in 0..self.total_nodes {
                    if (self.adj_list_rev[i] >> neighbor) & 1 != 0 {
                        self.in_degree[neighbor] -= 1;
                    }
                }
                return true;
            }
        }
        false
    }

    fn backtrack(&mut self) -> bool {
        if self.path_stack_len > 0 {
            self.path_stack_len -= 1;
            let last_added_idx_u8 = self.path_stack[self.path_stack_len as usize];
            let last_added_idx = last_added_idx_u8 as usize;

            if self.current_permutation_len > 0 {
                self.current_permutation_len -= 1;
                self.current_permutation[self.current_permutation_len as usize] = b"";
            }

            self.used.clear(last_added_idx).unwrap();

            for neighbor in 0..self.total_nodes {
                if (self.adj_list_rev[last_added_idx] >> neighbor) & 1 != 0 {
                    self.in_degree[neighbor] += 1;
                }
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
    type Item = [&'a [u8]; N];

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }

        loop {
            if self.current_permutation_len as usize == self.total_nodes {
                let result = self.current_permutation;
                self.backtrack();
                return Some(result);
            }

            // Try to extend the current partial permutation
            if self.try_extend() {
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
