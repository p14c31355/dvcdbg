//! # I2C Command Sequence Explorer
//!
//! This module provides an algorithm to discover valid sequences of I2C commands
//! for devices with dependency constraints.
//!
//! ## Overview
//! - `Explorer` holds a sequence of `CmdNode`s, each representing a command and its dependencies.
//! - The `explore` function performs:
//!   1. **Iterative staging**: topological sort-like process to place commands with satisfied dependencies.
//!   2. **Permutation exploration**: non-recursive, linear-stack-based exploration of unresolved commands.
//!
//! ## Usage
//! ```ignore
//! use crate::{Explorer, CmdNode, I2cCompat};
//!
//! struct MyExecutor;
//! impl<I2C: I2cCompat> CmdExecutor<I2C> for MyExecutor {
//!     fn exec(&mut self, i2c: &mut I2C, addr: u8, cmd: &[u8]) -> bool {
//!         let mut buf = [0x00, 0x00];
//!         buf[1] = cmd[0];
//!         i2c.write(addr, &buf).is_ok()
//!     }
//! }
//!
//! let cmds = &[
//!     CmdNode { bytes: &[0x01], deps: &[] },
//!     CmdNode { bytes: &[0x02], deps: &[0x01] },
//!     CmdNode { bytes: &[0x03], deps: &[0x01] },
//! ];
//! let explorer = Explorer { sequence: cmds };
//! let mut executor = MyExecutor;
//! // explorer.explore(&mut i2c, &mut serial, &mut executor).unwrap();
//! ```
//!
//! ## AVR / Embedded Constraints
//! - **Stack-safe**: The permutation algorithm is iterative to avoid stack overflow on devices with tiny stacks (e.g., AVR).
//! - **RAM Usage**: `heapless::Vec` is used for `path_stack`, `loop_start_indices`, and `current`, while `current_set` and `used` are fixed-size arrays.
//!   These consume RAM proportional to the number of unresolved commands or the `CMD_CAPACITY` constant. Limit `CMD_CAPACITY` to a safe number (e.g., 8–16) for 8-bit MCUs to manage static memory allocation.
//! - **Performance**: Unresolved commands are explored in factorial order (`n!`). Keep unresolved command count low to avoid long execution times.
//! - **Error Handling**: I2C write errors are will be discarded. It is recommended to use scan_init_sequence() first.
//!
//! ## Notes
//! - The algorithm ensures **dependency order is respected**.
//! - Commands are staged and permuted only when dependencies allow.
//! - The non-recursive approach is chosen to make the algorithm safer for small-memory MCUs.

use crate::scanner::{I2C_SCAN_ADDR_END, I2C_SCAN_ADDR_START};
use heapless::Vec;

const CMD_CAPACITY: usize = 32;
const MAX_PERMUTATION_WARNING_THRESHOLD: usize = 8;
const I2C_ADDRESS_COUNT: usize = 128;

/// Errors that can occur during exploration of command sequences.
pub enum ExplorerError {
    /// The provided sequence contained more commands than supported (`CMD_CAPACITY`).
    TooManyCommands,
}

enum BacktrackReason {
    FoundPermutation, // A full, valid sequence was found
    ExhaustedOptions, // Failed to extend the current partial sequence
}

/// Represents a single I2C command in the dependency graph.
///
/// Each command may depend on other commands, meaning they must appear
/// earlier in the sequence before this command can be executed.
pub struct CmdNode<'a> {
    /// The I2C command bytes to be sent. Can be a single command or a command with parameters.
    pub bytes: &'a [u8],
    /// The list of command bytes that must precede this command. The dependency is on the *first* byte of the dependent command.
    pub deps: &'a [u8],
}

/// An explorer that attempts to discover valid I2C command sequences
/// given a list of commands with dependencies.
///
/// The algorithm:
/// - First performs a topological sort of commands with no unresolved dependencies.
/// - Then, for the remaining commands, iteratively generates permutations
///   that satisfy all dependency constraints.
/// - For each candidate sequence, attempts it on all I2C addresses in the scan range.
pub struct Explorer<'a> {
    /// The input sequence of command nodes (with dependencies).
    pub sequence: &'a [CmdNode<'a>],
}

/// Internal state used during permutation generation.
///
/// This struct is not exposed publicly, but its fields are documented
/// to aid maintainers:
///
/// - `current`: the sequence of command bytes being built so far.
/// - `used`: flags marking which unresolved command indices are currently in `current`.
/// - `current_set`: boolean lookup for whether a specific command byte is in `current`.
/// - `path_stack`: stack of indices into `unresolved`, representing the order of decisions.
/// - `loop_start_indices`: optimization to avoid retrying candidates already attempted at each recursion depth.
struct PermutationState<'a, const C: usize> {
    current: Vec<&'a [u8], C>,
    used: [bool; C],
    current_set: [bool; 256],
    path_stack: Vec<usize, C>,
    loop_start_indices: Vec<usize, C>,
}

/// A trait for executing a command on an I2C bus.
///
/// This abstraction allows the `Explorer` to be decoupled from the specific
/// I2C protocol (e.g., adding a control byte like `0x00`).
pub trait CmdExecutor<I2C> {
    /// Executes a given command byte sequence on the specified I2C address.
    /// Returns `true` on success, `false` otherwise.
    fn exec(&mut self, i2c: &mut I2C, addr: u8, cmd: &[u8]) -> bool;
}

impl<'a> Explorer<'a> {
    /// Explore valid I2C command sequences for the provided command graph.
    ///
    /// # Parameters
    /// - `i2c`: An I2C implementation used to test candidate sequences against device addresses.
    /// - `serial`: A serial writer for logging progress and results.
    /// - `executor`: The object responsible for executing a single command on the bus.
    ///
    /// # Returns
    /// - `Ok(())` if exploration ran to completion.
    /// - `Err(ExplorerError::TooManyCommands)` if the input sequence exceeded capacity.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use your_crate::{Explorer, CmdNode};
    ///
    /// // Two commands: 0xA0 depends on 0x90, 0x90 has no deps.
    /// let nodes = [
    ///     CmdNode { bytes: &[0x90], deps: &[] },
    ///     CmdNode { bytes: &[0xA0], deps: &[0x90] },
    /// ];
    ///
    /// let explorer = Explorer { sequence: &nodes };
    ///
    /// // Dummy I2C + Serial + Executor implementations would be injected here in real use.
    /// // explorer.explore(&mut i2c, &mut serial, &mut executor);
    /// ```
    /// # Notes
    /// - This function may take a very long time if many commands remain unresolved,
    ///   since it must try permutations of them.
    /// - Successfully discovered addresses are logged to the provided `serial` writer.
    pub fn explore<I2C, W, E>(
        &self,
        i2c: &mut I2C,
        serial: &mut W,
        executor: &mut E,
    ) -> Result<(), ExplorerError>
    where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
        E: CmdExecutor<I2C>,
    {
        let mut staged: Vec<&'a [u8], CMD_CAPACITY> = Vec::new();
        if self.sequence.len() > CMD_CAPACITY {
            let _ = writeln!(serial, "error: too many commands");
            return Err(ExplorerError::TooManyCommands);
        }

        // Build initial sequence of commands with all dependencies satisfied
        let mut remaining: Vec<usize, CMD_CAPACITY> = (0..self.sequence.len()).collect();
        let mut staged_set = [false; 256];

        loop {
            let before = staged.len();
            remaining.retain(|&idx| {
                let node = &self.sequence[idx];
                if node.deps.iter().all(|d| staged_set[*d as usize]) {
                    staged
                        .push(node.bytes)
                        .expect("staged vec should have enough capacity");
                    if let Some(first_byte) = node.bytes.first() {
                        staged_set[*first_byte as usize] = true;
                    }
                    false
                } else {
                    true
                }
            });
            if staged.len() == before {
                break;
            }
        }

        if !remaining.is_empty() {
            let _ = writeln!(
                serial,
                "[explorer] warning: unresolved dependencies found, possibly due to a cycle."
            );
        }

        let _ = writeln!(serial, "[explorer] staged:");
        self.write_sequence(serial, &staged);
        let _ = writeln!(serial, "[explorer] unresolved: {remaining:?}");

        let mut current_state = PermutationState {
            current: staged,
            used: [false; CMD_CAPACITY],
            current_set: staged_set,
            path_stack: Vec::new(),
            loop_start_indices: Vec::from_slice(&[0]).unwrap(),
        };
        let mut solved_addrs = [false; I2C_ADDRESS_COUNT];

        if remaining.len() > MAX_PERMUTATION_WARNING_THRESHOLD {
            let _ = writeln!(
                serial,
                "[explorer] warning: Large number of unresolved commands ({}). This may take a very long time.",
                remaining.len()
            );
        }

        self.permute(
            i2c,
            serial,
            &remaining,
            &mut current_state,
            &mut solved_addrs,
            executor,
        );

        Ok(())
    }

    fn permute<I2C, W, E>(
        &self,
        i2c: &mut I2C,
        serial: &mut W,
        unresolved: &Vec<usize, CMD_CAPACITY>,
        state: &mut PermutationState<'a, CMD_CAPACITY>,
        solved_addrs: &mut [bool; I2C_ADDRESS_COUNT],
        executor: &mut E,
    ) where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
        E: CmdExecutor<I2C>,
    {
        'main_loop: loop {
            if state.current.len() == self.sequence.len() {
                self.handle_full_permutation(i2c, serial, state, solved_addrs, executor);
                if !self.backtrack(unresolved, state, BacktrackReason::FoundPermutation) {
                    break 'main_loop;
                }
            } else if !self.try_extend_permutation(unresolved, state) {
                // Could not extend, backtrack
                if !self.backtrack(unresolved, state, BacktrackReason::ExhaustedOptions) {
                    break 'main_loop;
                }
            }
        }
    }

    /// Called whenever a full valid permutation has been generated.
    ///
    /// Attempts the sequence against all possible I2C addresses,
    /// marking those that succeed and logging the result.
    fn handle_full_permutation<I2C, W, E>(
        &self,
        i2c: &mut I2C,
        serial: &mut W,
        state: &mut PermutationState<'a, CMD_CAPACITY>,
        solved_addrs: &mut [bool; I2C_ADDRESS_COUNT],
        executor: &mut E,
    ) where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
        E: CmdExecutor<I2C>,
    {
        let _ = writeln!(serial, "[explorer] candidate:");
        self.write_sequence(serial, &state.current);

        for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
            if solved_addrs[addr as usize] {
                continue;
            }
            let all_ok = state
                .current
                .iter()
                .all(|&cmd| executor.exec(i2c, addr, cmd));
            if all_ok {
                let _ = writeln!(
                    serial,
                    "[explorer] success: sequence works for addr 0x{addr:02X}"
                );
                solved_addrs[addr as usize] = true;
            }
        }
    }

    /// Attempts to extend the current partial permutation by adding
    /// one more command that satisfies its dependencies.
    ///
    /// Returns `true` if a command was added, or `false` if no valid candidate was found.
    fn try_extend_permutation(
        &self,
        unresolved: &Vec<usize, CMD_CAPACITY>,
        state: &mut PermutationState<'a, CMD_CAPACITY>,
    ) -> bool {
        let current_loop_start_idx = *state.loop_start_indices.last().unwrap();
        for (pos, &idx) in unresolved.iter().enumerate().skip(current_loop_start_idx) {
            if state.used[pos] {
                continue;
            }
            let node = &self.sequence[idx];
            if node.deps.iter().all(|d| state.current_set[*d as usize]) {
                // Make choice
                state.current.push(node.bytes).unwrap();
                if let Some(first_byte) = node.bytes.first() {
                    state.current_set[*first_byte as usize] = true;
                }
                state.used[pos] = true;

                let _ = state.path_stack.push(pos);
                let _ = state.loop_start_indices.push(0);
                return true;
            }
        }
        false
    }

    /// Backtracks to the previous decision point in the permutation search.
    ///
    /// # Parameters
    /// - `reason`: Indicates why backtracking is occurring. This determines how the search state is updated.
    ///   - `BacktrackReason::FoundPermutation`: A full permutation was found. The search continues for the next sibling.
    ///   - `BacktrackReason::ExhaustedOptions`: The current path cannot be extended. The search backtracks and prunes this branch.
    ///
    /// Returns `true` if backtracking can continue, or `false` if the root was reached.
    fn backtrack(
        &self,
        unresolved: &Vec<usize, CMD_CAPACITY>,
        state: &mut PermutationState<'a, CMD_CAPACITY>,
        _reason: BacktrackReason,
    ) -> bool {
        if let Some(last_added_pos) = state.path_stack.pop() {
            let node = &self.sequence[unresolved[last_added_pos]];
            if let Some(first_byte) = node.bytes.first() {
                state.current_set[*first_byte as usize] = false;
            }
            state.used[last_added_pos] = false;
            state.current.pop();

            state.loop_start_indices.pop();

            if let Some(last_loop_idx) = state.loop_start_indices.last_mut() {
                *last_loop_idx += 1;
            } else {
                return false;
            }
            true
        } else {
            false
        }
    }

    fn hex_byte<W: core::fmt::Write>(w: &mut W, b: u8) {
        const HEX_CHARS: &[u8] = b"0123456789ABCDEF";
        let hi = HEX_CHARS[((b >> 4) & 0x0F) as usize];
        let lo = HEX_CHARS[(b & 0x0F) as usize];
        w.write_char(hi as char).ok();
        w.write_char(lo as char).ok();
    }

    fn write_sequence<W: core::fmt::Write>(&self, w: &mut W, seq: &[&[u8]]) {
        for bytes in seq {
            for &b in *bytes {
                Self::hex_byte(w, b);
            }
            w.write_char(' ').ok();
        }
        w.write_char('\n').ok();
    }
}