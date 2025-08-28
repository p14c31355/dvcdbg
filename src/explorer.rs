use heapless::Vec;
use heapless::index_map::FnvIndexMap;
use crate::scanner::{I2C_SCAN_ADDR_END, I2C_SCAN_ADDR_START};
const I2C_ADDRESS_COUNT: usize = 128;

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

pub struct Explorer<'a, const N: usize> {
    pub sequence: &'a [CmdNode],
}

pub struct ExploreResult {
    pub found_addrs: Vec<u8, I2C_ADDRESS_COUNT>,
    pub permutations_tested: usize,
}

impl<'a, const N: usize> Explorer<'a, N> {
    fn kahn_topo_sort(&self, visited: &Vec<bool, N>) -> Result<Vec<usize, N>, ExplorerError> {
        let mut in_degree: Vec<usize, N> = Vec::new();
        let mut adj_rev: Vec<Vec<usize, N>, N> = Vec::new();
        in_degree.resize(self.sequence.len(), 0).map_err(|_| ExplorerError::BufferOverflow)?;
        adj_rev.resize(self.sequence.len(), Vec::new()).map_err(|_| ExplorerError::BufferOverflow)?;

        for (i, node) in self.sequence.iter().enumerate() {
            in_degree[i] = node.deps.len();
            for &dep in node.deps.iter() {
                if dep >= self.sequence.len() {
                    return Err(ExplorerError::InvalidDependencyIndex);
                }
                adj_rev[dep].push(i).map_err(|_| ExplorerError::BufferOverflow)?;
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
        let mut solved_addrs: [bool; I2C_ADDRESS_COUNT] = [false; I2C_ADDRESS_COUNT];
        let mut permutation_count = 0;
        let mut visited_nodes: Vec<bool, N> = Vec::new();
        visited_nodes.resize(self.sequence.len(), false).map_err(|_| ExplorerError::BufferOverflow)?;
        let mut hash_table: FnvIndexMap<u64, (), N> = FnvIndexMap::new();

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
            hash_table.insert(hash, ()).map_err(|_| ExplorerError::BufferOverflow)?;
            permutation_count += 1;

            for addr_val in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                let addr_idx = addr_val as usize;
                if solved_addrs[addr_idx] {
                    continue;
                }

                let mut all_ok = true;
                for &idx in order.iter() {
                    if let Err(_) = executor.exec(i2c, addr_val, self.sequence[idx].bytes, logger) {
                        all_ok = false;
                        break;
                    }
                }

                if all_ok {
                    solved_addrs[addr_idx] = true;
                    found_addresses.push(addr_val).map_err(|_| ExplorerError::BufferOverflow)?;
                }
            }

            for &idx in order.iter() {
                visited_nodes[idx] = true;
            }
        }

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
    pub fn get_one_topological_sort_buf(
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
                "[dbg] Node {} deps={:?}, in_degree={}",
                i, node.deps, in_degree[i]
            )
            .ok();

            for &dep_idx in node.deps.iter() {
                if dep_idx >= len {
                    writeln!(
                        serial,
                        "[error] Node {} has invalid dep index {} (len={})",
                        i, dep_idx, len
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
        for i in 0..len {
            if in_degree[i] == 0 {
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
                "[dbg] Node {} bytes={:02X?} (len={})",
                i,
                &result_sequence[i][..result_len_per_node[i]],
                result_len_per_node[i]
            )
            .ok();
        }

        Ok((result_sequence, result_len_per_node))
    }
}
