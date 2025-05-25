use super::Instruction;

/// Cop0 register structure.
#[derive(Debug, Default)]
pub struct Cop0 {
    /// r3 - Breakpoint Program Counter
    pub bpc: u32,

    /// r5 - Breakpoint Data Address
    pub bda: u32,

    /// r6 - Target Address
    pub tar: u32,

    /// r7 - Debug and Cache Invalidate Control
    pub dcic: u32,

    /// r8 - Bad Address
    pub bada: u32,

    /// r9 - Breakpoint Data Address Mask
    pub bdma: u32,

    /// r11 - Breakpoint Program Counter Mask
    pub bpcm: u32,

    /// r12 - Status Register
    pub sr: u32,

    /// r13 - Cause of the last exception
    pub cause: u32,

    /// r14 - Exception Program Counter
    pub epc: u32,
}

// r15 - Processor ID, is a read-only register that contains the version of the
// processor. Most emulators hardcode it to 2, and so will we.
const PROCESSOR_ID: u32 = 2;

impl Cop0 {
    /// Creates a new Cop0 instance with all registers initialized to zero.
    pub fn new() -> Self {
        Cop0::default()
    }

    /// Reads a value from the specified Cop0 register.
    pub fn read(&self, reg: usize) -> Option<u32> {
        // Return the register based on the index
        match reg {
            3 => Some(self.bpc),
            5 => Some(self.bda),
            6 => Some(self.tar),
            7 => Some(self.dcic),
            8 => Some(self.bada),
            9 => Some(self.bdma),
            11 => Some(self.bpcm),
            12 => Some(self.sr),
            13 => Some(self.cause),
            14 => Some(self.epc),
            15 => Some(PROCESSOR_ID),
            _ => None, // The register does not exist on the PS1
        }
    }

    /// Writes a value to the specified Cop0 register.
    pub fn write(&mut self, reg: usize, value: u32) -> Result<(), ()> {
        println!("[Cop0] Writing to register {reg}: {value:x}");
        
        match reg {
            3 => self.bpc = value,
            5 => self.bda = value,
            6 => self.tar = value,
            7 => self.dcic = value,
            8 => self.bada = value,
            9 => self.bdma = value,
            11 => self.bpcm = value,
            12 => self.sr = value,
            13 => self.cause = value,
            14 => self.epc = value,
            15 => {} // Processor ID is read-only, do nothing
            _ => return Err(()) // The register does not exist on the PS1
        }

        Ok(())
    }

    pub fn execute(&mut self, instruction: Instruction) {
        unimplemented!("COP0 instruction: {:?}", instruction);
    }
}