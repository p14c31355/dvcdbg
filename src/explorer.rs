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

pub struct Explorer<'a, const N: usize> {
    pub sequence: &'a [CmdNode],
}

pub struct ExploreResult {
    pub found_addrs: Vec<u8, I2C_ADDRESS_COUNT>,
    pub permutations_tested: usize,
}

impl<'a, const N: usize> Explorer<'a, N> {
    fn kahn_topo_sort(&self, visited: &mut [bool; N]) -> Result<Vec<usize>, ExplorerError> {
        let mut in_degree = [0usize; N];
        let mut adj_rev: [Vec<usize, N>; N] = array_init::array_init(|_| Vec::new());
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
        let mut solved_addrs = [false; I2C_ADDRESS_COUNT];
        let mut permutation_count = 0;
        let mut visited_nodes = [false; N];
        let mut hash_table: FnvIndexMap<u64, (), N> = FnvIndexMap::new();

        loop {
            let order = self.kahn_topo_sort(&mut visited_nodes)?;
            if order.is_empty() {
                break;
            }

            let mut hasher = crc32fast::Hasher::new();
            for &idx in order.iter() {
                hasher.update(self.sequence[idx].bytes);
            }
            let hash = hasher.finalize();
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
                    if let Err(e) = executor.exec(i2c, addr_val, self.sequence[idx].bytes, logger) {
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
}
