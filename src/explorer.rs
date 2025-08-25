use crate::scanner::{I2C_SCAN_ADDR_END, I2C_SCAN_ADDR_START};
use heapless::Vec;

const CMD_CAPACITY: usize = 32;

/// Represents a single command node with an associated command byte (`cmd`)
/// and a list of dependencies (`deps`).  
///
/// A dependency is expressed as another command byte that must appear before
/// this command in any valid sequence.
///
/// This structure does not enforce ordering itself. The [`Explorer`] is
/// responsible for staging and permuting nodes according to dependencies.
pub struct CmdNode<'a> {
    /// The command byte to be written.
    pub cmd: u8,
    /// Dependencies (list of command bytes that must precede this one).
    pub deps: &'a [u8],
}

/// Explorer attempts to discover valid command orderings for I2C device
/// initialization by combining **dependency resolution** (staging) with
/// **permutation search** for unresolved nodes.
///
/// The workflow:
/// 1. **Staging phase**  
///    Commands with satisfied dependencies are placed into the `staged` list.
///    This behaves like a topological sort.
///
/// 2. **Permutation phase**  
///    Any commands that remain unresolved are permuted recursively to explore
///    all possible valid orderings. Each candidate sequence is tested against
///    the I2C bus and results are logged to the serial output.
///
/// ## Notes
/// - Limited to `CMD_CAPACITY` commands (default: 32).
/// - Logs candidates and I2C errors to the provided serial writer.
/// - Intended for device bring-up and debugging, not production.
pub struct Explorer<'a> {
    /// The sequence of commands with dependency metadata.
    pub sequence: &'a [CmdNode<'a>],
}

impl<'a> Explorer<'a> {
    /// Explore possible valid orderings of the command sequence.
    ///
    /// 1. Performs iterative staging (similar to a topological sort)
    ///    to collect commands whose dependencies are satisfied.
    /// 2. For unresolved commands, invokes [`Self::permute`] to try
    ///    all possible valid permutations.
    ///
    /// Each fully formed candidate sequence is written to the I2C bus
    /// at addresses `I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END`.
    ///
    /// # Errors
    /// Returns `Err(())` if:
    /// - The number of commands exceeds `CMD_CAPACITY`.
    /// - Any I2C error is encountered (logged and continued).
    pub fn explore<I2C, W>(&self, i2c: &mut I2C, serial: &mut W) -> Result<(), ()>
    where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
    {
        // Iterative staging (topological sort–like approach)
        let mut staged: Vec<u8, CMD_CAPACITY> = Vec::new();

        if self.sequence.len() > CMD_CAPACITY {
            let _ = writeln!(serial, "error: too many commands");
            return Err(());
        }

        let mut remaining: Vec<usize, CMD_CAPACITY> = (0..self.sequence.len()).collect();
        let mut staged_set = [false; 256];

        loop {
            let before = staged.len();
            remaining.retain(|&idx| {
                let node = &self.sequence[idx];
                if node.deps.iter().all(|d| staged_set[*d as usize]) {
                    // Dependency satisfied, stage this command
                    staged.push(node.cmd).unwrap();
                    staged_set[node.cmd as usize] = true;
                    false // remove from remaining
                } else {
                    true // keep in remaining
                }
            });

            if staged.len() == before {
                // No progress in this iteration, stop staging
                break;
            }
        }

        let _ = writeln!(serial, "[explorer] staged: {:?}", staged);
        let _ = writeln!(serial, "[explorer] unresolved: {:?}", remaining);

        // Start permutation search with unresolved commands
        let mut current: Vec<u8, CMD_CAPACITY> = staged.clone();
        let mut used = [false; CMD_CAPACITY];
        let mut current_set = [false; 256];
        for &cmd in current.iter() {
            current_set[cmd as usize] = true;
        }

        self.permute(i2c, serial, &remaining, &mut current, &mut used, &mut current_set)?;
        Ok(())
    }

    /// Recursive permutation search for unresolved commands.
    ///
    /// At each recursive step:
    /// - If all commands are placed, a candidate sequence is logged and tested.
    /// - Otherwise, commands whose dependencies are satisfied are added,
    ///   and the function recurses further.
    ///
    /// Backtracking ensures all possible valid orderings are explored.
    fn permute<I2C, W>(
        &self,
        i2c: &mut I2C,
        serial: &mut W,
        unresolved: &Vec<usize, CMD_CAPACITY>,
        current: &mut Vec<u8, CMD_CAPACITY>,
        used: &mut [bool; CMD_CAPACITY],
        current_set: &mut [bool; 256],
    ) -> Result<(), ()>
    where
        I2C: crate::compat::I2cCompat,
        W: core::fmt::Write,
    {
        if current.len() == self.sequence.len() {
            let _ = writeln!(serial, "[explorer] candidate: {:?}", current);

            // Test candidate sequence by writing each command to I2C
            for &cmd in current.iter() {
                for addr in I2C_SCAN_ADDR_START..=I2C_SCAN_ADDR_END {
                    if let Err(e) = i2c.write(addr, &[cmd]) {
                        let _ = writeln!(serial, "i2c error at addr 0x{:02X}: {:?}", addr, e);
                    }
                }
            }
            return Ok(());
        }

        for (pos, &idx) in unresolved.iter().enumerate() {
            if used[pos] {
                continue;
            }
            let node = &self.sequence[idx];
            if node.deps.iter().all(|d| current_set[*d as usize]) {
                // Add this command
                current.push(node.cmd).unwrap();
                current_set[node.cmd as usize] = true;
                used[pos] = true;

                self.permute(i2c, serial, unresolved, current, used, current_set)?;

                // Backtrack
                used[pos] = false;
                current_set[node.cmd as usize] = false;
                current.pop();
            }
        }
        Ok(())
    }

    // === Utility functions for ASCII hex printing ===

    /// Converts a single byte into its uppercase hexadecimal ASCII form
    /// and writes it to the given writer.
    fn hex_byte<W: core::fmt::Write>(w: &mut W, b: u8) {
        const HEX_CHARS: &[u8] = b"0123456789ABCDEF";
        let hi = HEX_CHARS[((b >> 4) & 0x0F) as usize];
        let lo = HEX_CHARS[(b & 0x0F) as usize];
        w.write_char(hi as char).ok();
        w.write_char(lo as char).ok();
    }

    /// Writes a sequence of bytes as space-separated uppercase hex values
    /// followed by a newline.
    fn write_sequence<W: core::fmt::Write>(&self, w: &mut W, seq: &[u8]) {
        for &b in seq {
            Self::hex_byte(w, b);
            w.write_char(' ').ok();
        }
        w.write_char('\n').ok();
    }
}
