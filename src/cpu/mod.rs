//[ cpu-new
use crate::ram::Ram;

pub enum AccessSize {
    Byte,
    HalfWord,
    Word,
}

const NUM_REGISTERS: usize = 32;

/// The emulated PS1 CPU
pub struct Cpu {
    pub registers: [u32; NUM_REGISTERS],
    pub pc: u32,

    pub ram: Ram,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            registers: [0; NUM_REGISTERS],
            pc: 0,
            ram: Ram::new(),
        }
    }
}
//] cpu-new
