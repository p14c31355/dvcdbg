// explorer.rs
use crate::compat::ascii;
use core::fmt::Write;
use heapless::{FnvIndexMap, Vec};

use crate::scanner::{I2C_SCAN_ADDR_END, I2C_SCAN_ADDR_START};
const I2C_ADDRESS_COUNT: usize = 128;
pub const LOG_BUFFER_CAPACITY: usize = 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum ExplorerError {
    TooManyCommands,
    DependencyCycle,
    NoValidAddressesFound,
    ExecutionFailed,
    BufferOverflow,
    InvalidDependencyIndex,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExecutorError {
    I2cError(crate::error::ErrorKind),
    ExecFailed,
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

pub trait CmdExecutor<I2C> {
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

pub struct Explorer<'a, const N: usize, const MAX_CMD_LEN: usize> {
    pub sequence: &'a [CmdNode],
}

pub struct PermutationIter<'a, const N: usize> {
    sequence: &'a [CmdNode],
    current_permutation: Vec<&'a [u8], N>,
    used: [bool; N],
    in_degree: Vec<usize, N>,
    adj_list_rev: Vec<Vec<usize, N>, N>,
    path_stack: Vec<usize, N>,
    loop_start_indices: Vec<usize, N>,
    is_done: bool,
    total_nodes: usize,
}

pub struct ExploreResult {
    pub found_addrs: Vec<u8, I2C_ADDRESS_COUNT>,
    pub permutations_tested: usize,
}

impl<'a, const N: usize, const MAX_CMD_LEN: usize> Explorer<'a, N, MAX_CMD_LEN> {
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
        max_len + 1
    }

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
            initial_in_degree[i] = node.deps.len();
            for &dep_idx in node.deps.iter() {
                if dep_idx >= self.sequence.len() {
                    return Err(ExplorerError::InvalidDependencyIndex);
                }
                adj_list_rev[dep_idx]
                    .push(i)
                    .map_err(|_| ExplorerError::BufferOverflow)?;
            }
        }

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
            used: [false; N],
            in_degree: initial_in_degree,
            adj_list_rev,
            path_stack: Vec::new(),
            loop_start_indices: Vec::new(),
            is_done: false,
            total_nodes: self.sequence.len(),
        })
    }

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
        if self.sequence.is_empty() {
            logger.log_info("[explorer] No commands provided.");
            return Err(ExplorerError::NoValidAddressesFound);
        }

        let mut found_addresses: Vec<u8, I2C_ADDRESS_COUNT> = Vec::new();
        let mut solved_addrs = [false; I2C_ADDRESS_COUNT];
        let mut permutation_count = 0;
        let mut hash_table: FnvIndexMap<u64, (), N> = FnvIndexMap::new();

        let mut iter = self.permutations()?;
        logger.log_info("[explorer] Starting permutation exploration...");

        while let Some(sequence) = iter.next() {
            let mut hasher = crc32fast::Hasher::new();
            for &cmd in sequence.iter() {
                hasher.update(cmd);
            }
            let hash = hasher.finalize();
            if hash_table.contains_key(&hash) {
                continue;
            }
            hash_table.insert(hash, ()).map_err(|_| ExplorerError::BufferOverflow)?;

            permutation_count += 1;

            for addr_val in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                let addr_idx = addr_val as usize;
                if solved_addrs[addr_idx] {
                    continue;
                }

                let mut all_ok = true;
                for &cmd in sequence.iter() {
                    if let Err(e) = executor.exec(i2c, addr_val, cmd, logger) {
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
}

impl<'a, const N: usize> Iterator for PermutationIter<'a, N> {
    type Item = Vec<&'a [u8], N>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }

        loop {
            if self.current_permutation.len() == self.total_nodes {
                let full_sequence = self.current_permutation.clone();
                if !self.backtrack() {
                    self.is_done = true;
                }
                return Some(full_sequence);
            }

            if self.try_extend() {
                continue;
            } else {
                if !self.backtrack() {
                    self.is_done = true;
                    return None;
                }
            }
        }
    }
}

impl<'a, const N: usize> PermutationIter<'a, N> {
    fn try_extend(&mut self) -> bool {
        let start_idx_for_level = self.loop_start_indices.last().copied().unwrap_or(0);

        for idx in start_idx_for_level..self.total_nodes {
            if self.used[idx] {
                continue;
            }

            if self.in_degree[idx] == 0 {
                self.current_permutation
                    .push(self.sequence[idx].bytes)
                    .unwrap();
                self.used[idx] = true;
                for &dependent_idx in self.adj_list_rev[idx].iter() {
                    self.in_degree[dependent_idx] -= 1;
                }
                self.path_stack.push(idx).unwrap();
                self.loop_start_indices.push(idx + 1).unwrap();
                return true;
            }
        }
        false
    }

    fn backtrack(&mut self) -> bool {
        if let Some(last_added_idx) = self.path_stack.pop() {
            self.current_permutation.pop();
            self.used[last_added_idx] = false;
            for &dependent_idx in self.adj_list_rev[last_added_idx].iter() {
                self.in_degree[dependent_idx] += 1;
            }
            self.loop_start_indices.pop();
            if self.path_stack.is_empty() {
                self.is_done = true;
                return false;
            }
            true
        } else {
            self.is_done = true;
            false
        }
    }
}
