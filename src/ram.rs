//[ ram-new
pub const RAM_SIZE: usize = 2 * 1024 * 1024;

/// A RAM (Random Access Memory) structure that simulates a memory space,
/// with little-endian accessors for reading and writing bytes.
pub struct Ram {
    data: Vec<u8>,
}

impl Ram {
    pub fn new() -> Self {
        Self {
            data: vec![0; RAM_SIZE],
        }
    }
}
//] ram-new
