// src/compat/buffer.rs
pub const fn calculate_cmd_buffer_size(num_commands: usize, max_cmd_len: usize) -> usize {
    num_commands * (max_cmd_len + 1) + num_commands * 2
}

pub const ERROR_STRING_BUFFER_SIZE: usize = 768;
