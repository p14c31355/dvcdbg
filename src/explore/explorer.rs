// explorer.rs

use crate::compat::err_compat::HalErrorExt;
use crate::compat::util;
use crate::error::{ExecutorError, ExplorerError};
use heapless::Vec;

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

/// A stateful iterator for generating a single topological sort using Kahn's algorithm.
/// This avoids allocating the entire sorted sequence in memory at once.
pub struct TopologicalIter<'a, const N: usize, const MAX_DEPS_TOTAL: usize> {
    nodes: &'a [CmdNode],
    in_degree: [u8; N],
    adj_list_rev_flat: [u8; MAX_DEPS_TOTAL],
    adj_list_rev_offsets: [u16; N],
    queue: heapless::Vec<u8, N>,
    visited_count: usize,
    total_non_failed: usize,
}

impl<'a, const N: usize, const MAX_DEPS_TOTAL: usize> TopologicalIter<'a, N, MAX_DEPS_TOTAL> {
    const _ASSERT_N_LE_256: () = assert!(
        N <= 256,
        "TopologicalIter uses a u8 bitmask, so N cannot exceed 256"
    );

    pub fn new(
        explorer: &'a Explorer<N, MAX_DEPS_TOTAL>,
        failed_nodes: &util::BitFlags,
    ) -> Result<Self, ExplorerError> {
        let len = explorer.nodes.len();
        if len > N {
            return Err(ExplorerError::TooManyCommands);
        }

        let mut in_degree: [u8; N] = [0; N];
        let mut adj_list_rev_offsets: [u16; N] = [0; N];
        let mut adj_list_rev_flat: [u8; MAX_DEPS_TOTAL] = [0; MAX_DEPS_TOTAL];
        let mut total_non_failed = 0;

        // Pass 1: Calculate in-degrees and build the offsets for the flat array.
        let mut rev_adj_counts = [0u16; N];
        for (i, node) in explorer.nodes.iter().enumerate().take(len) {
            if !failed_nodes.get(i).unwrap_or(false) {
                total_non_failed += 1;
                for &dep_idx in node.deps.iter() {
                    let dep_idx_usize = dep_idx as usize;
                    if dep_idx_usize >= N {
                        return Err(ExplorerError::InvalidDependencyIndex);
                    }
                    in_degree[i] = in_degree[i].saturating_add(1);
                    rev_adj_counts[dep_idx_usize] = rev_adj_counts[dep_idx_usize].saturating_add(1);
                }
            }
        }

        let mut current_offset: u16 = 0;
        for i in 0..len {
            adj_list_rev_offsets[i] = current_offset;
            current_offset = current_offset.saturating_add(rev_adj_counts[i]);
        }
        if current_offset as usize > MAX_DEPS_TOTAL {
            return Err(ExplorerError::BufferOverflow);
        }

        // Pass 2: Populate the flat array using the pre-calculated offsets.
        let mut write_pointers = adj_list_rev_offsets.clone();
        for (i, node) in explorer.nodes.iter().enumerate().take(len) {
            if failed_nodes.get(i).unwrap_or(false) {
                continue;
            }
            for &dep_idx in node.deps.iter() {
                let dep_idx_usize = dep_idx as usize;
                let write_pos = write_pointers[dep_idx_usize] as usize;
                adj_list_rev_flat[write_pos] = i as u8;
                write_pointers[dep_idx_usize] = write_pointers[dep_idx_usize].saturating_add(1);
            }
        }

        let mut queue: heapless::Vec<u8, N> = heapless::Vec::new();
        for i in 0..len {
            if in_degree[i] == 0 && !failed_nodes.get(i).unwrap_or(false) {
                queue
                    .push(i as u8)
                    .map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

        Ok(Self {
            nodes: explorer.nodes,
            in_degree,
            adj_list_rev_flat,
            adj_list_rev_offsets,
            queue,
            visited_count: 0,
            total_non_failed,
        })
    }

    /// Checks if a cycle was detected after the iteration is complete.
    pub fn is_cycle_detected(&self) -> bool {
        self.visited_count != self.total_non_failed
    }
}

impl<'a, const N: usize, const MAX_DEPS_TOTAL: usize> Iterator
    for TopologicalIter<'a, N, MAX_DEPS_TOTAL>
{
    type Item = usize; // Return the index of the next node

    fn next(&mut self) -> Option<Self::Item> {
        if self.queue.is_empty() {
            return None;
        }

        let u_u8 = self.queue.pop().unwrap();
        let u = u_u8 as usize;
        self.visited_count += 1;

        let start_offset = self.adj_list_rev_offsets[u] as usize;
        let end_offset = if u + 1 < self.adj_list_rev_offsets.len() {
            self.adj_list_rev_offsets[u + 1] as usize
        } else {
            MAX_DEPS_TOTAL
        };

        // Process neighbors of 'u'
        for &v_u8 in &self.adj_list_rev_flat[start_offset..end_offset] {
            let v = v_u8 as usize;
            self.in_degree[v] = self.in_degree[v].saturating_sub(1);
            if self.in_degree[v] == 0 {
                if self.queue.push(v_u8).is_err() {
                    unreachable!("TopologicalIter queue overflowed");
                }
            }
        }

        Some(u)
    }
}

/// A command executor that prepends a prefix to each command.
pub struct PrefixExecutor<const INIT_SEQUENCE_LEN: usize, const CMD_BUFFER_SIZE: usize> {
    buffer: [u8; CMD_BUFFER_SIZE],
    buffer_len: usize,
    initialized_addrs: util::BitFlags,
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
            initialized_addrs: util::BitFlags::new(),
            prefix,
            init_sequence: init_seq_arr,
            init_sequence_len: init_seq_len,
        }
    }

    fn short_delay() {
        for _ in 0..1_000 {
            core::hint::spin_loop();
        }
    }

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
    util::prevent_garbled(
        writer,
        format_args!("[explorer] Sending node {cmd_idx} bytes: {cmd_bytes:02X?} ..."),
    );

    match executor.exec(i2c, addr, cmd_bytes, writer) {
        Ok(_) => {
            util::prevent_garbled(writer, format_args!("[explorer] OK"));
            Ok(())
        }
        Err(e) => {
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
        static NODES: &[$crate::explore::explorer::CmdNode] = &[
            $(
                $crate::explore::explorer::CmdNode {
                    bytes: &[ $( $b ),* ],
                    deps: &[ $( $( $d ),* )? ],
                }
            ),*
        ];

        const MAX_CMD_LEN_INTERNAL: usize = {
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
        const MAX_DEPS_TOTAL_INTERNAL: usize = {
            let mut total_deps = 0;
            let mut i = 0;
            while i < NODES.len() {
                total_deps += NODES[i].deps.len();
                i += 1;
            }
            total_deps
        };

        static EXPLORER: $crate::explore::explorer::Explorer<{NODES.len()}, {MAX_DEPS_TOTAL_INTERNAL}> =
            $crate::explore::explorer::Explorer::new(NODES);

        (
            &EXPLORER,
            $crate::explore::explorer::PrefixExecutor::<0, MAX_CMD_LEN_INTERNAL>::new($prefix, &[])
        )
    }};
}

/// simple macro to count comma-separated expressions at compile time
#[macro_export]
macro_rules! count_exprs {
    () => (0usize);
    ($x:expr $(, $xs:expr)*) => (1usize + $crate::count_exprs!($($xs),*));
}

pub struct Explorer<const N: usize, const MAX_DEPS_TOTAL: usize> {
    pub(crate) nodes: &'static [CmdNode],
}

pub struct ExploreResult {
    pub found_addrs: [u8; I2C_ADDRESS_COUNT],
    pub found_addrs_len: usize,
    pub permutations_tested: usize,
}

impl<const N: usize, const MAX_DEPS_TOTAL: usize> Explorer<N, MAX_DEPS_TOTAL> {
    pub fn topological_iter<'a>(
        &'a self,
        failed_nodes: &'a util::BitFlags,
    ) -> Result<TopologicalIter<'a, N, MAX_DEPS_TOTAL>, ExplorerError> {
        TopologicalIter::new(self, failed_nodes)
    }

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
}
