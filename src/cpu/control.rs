use crate::cpu::branch;

use super::Instruction;
use super::control_types::*;

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
    pub bad_vaddr: u32,

    /// r9 - Breakpoint Data Address Mask
    pub bdma: u32,

    /// r11 - Breakpoint Program Counter Mask
    pub bpcm: u32,

    /// r12 - Status Register
    pub status: Status,

    /// r13 - Cause of the last exception
    pub cause: Cause,

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
            8 => Some(self.bad_vaddr),
            9 => Some(self.bdma),
            11 => Some(self.bpcm),
            12 => Some(self.status.0),
            13 => Some(self.cause.0),
            14 => Some(self.epc),
            15 => Some(PROCESSOR_ID),
            _ => None, // The register does not exist on the PS1
        }
    }

    /// Writes a value to the specified Cop0 register.
    pub fn write(&mut self, reg: usize, value: u32) -> Result<(), ()> {
        // println!("[Cop0] Writing to register {reg}: {value:x}");

        match reg {
            3 => self.bpc = value,
            5 => self.bda = value,
            6 => self.tar = value,
            7 => self.dcic = value,
            8 => self.bad_vaddr = value,
            9 => self.bdma = value,
            11 => self.bpcm = value,
            12 => self.status.0 = value,
            13 => {
                // Only allow writing to bits 8 and 9 of the Cause Register
                let old_value = self.cause.0 & !0x0300; // Clear bits 8 and 9
                let new_value = value & 0x0300; // Keep only bits 8 and 9

                self.cause.0 = old_value | new_value;
            }
            14 => self.epc = value,
            15 => {}             // Processor ID is read-only, do nothing
            _ => return Err(()), // The register does not exist on the PS1
        }

        Ok(())
    }

    pub fn execute(&mut self, instruction: Instruction) {
        if instruction.cop_instruction() == 0x10 {
            // RFE: shift the low 6 bits of the Status Register by 2, then set
            // them again. KUo and IEo are copied, but left unchanged.
            let low_fields = self.status.low_fields();
            self.status
                .set_low_fields(low_fields & 0x30 | (low_fields >> 2));
        } else {
            panic!(
                "[Cop0] Unimplemented coprocessor instruction: {:#x}",
                instruction.cop_instruction()
            );
        }
    }

    /// Returns true if an interrupt should be handled.
    pub fn should_interrupt(&self) -> bool {
        // Check if the IE bit is set in the Status Register
        if !self.status.interrupt_enable() {
            return false;
        }

        self.cause.interrupt_pending() & self.status.interrupt_mask() != 0
    }

    /// Sets or clears the hardware interrupt bit (IP2) in the Cause register.
    pub fn set_hardware_interrupt(&mut self, value: bool) {
        self.cause.set_ip2(value);
    }

    /// Sets up the coprocessor registers to handle an exception, and returns
    /// the PC address to jump to for the exception handler.
    ///
    /// This method updates the Status Register, Cause Register, and Exception
    /// Program Counter (EPC).
    ///
    /// The `cause` parameter specifies the type of exception, and the `pc`
    /// parameter is the program counter that caused the exception.
    ///
    /// The `bds` parameter indicates whether the exception occurred in a
    /// branch delay slot.
    pub fn start_exception(
        &mut self,
        cause: ExceptionCause,
        pc: u32,
        npc: u32,
        in_bds: bool,
        branch_taken: bool,
        coprocessor_number: u32,
    ) -> u32 {
        // Copy the low 4 bits into bits 6-4 of the Status Register
        self.status.set_low_fields(self.status.low_fields() << 2);

        if in_bds {
            self.cause.set_branch_delay(true);
            self.cause.set_branch_taken(branch_taken);
            self.tar = npc;
            self.epc = pc.wrapping_sub(4);
        } else {
            self.cause.set_branch_delay(false);
            self.cause.set_branch_taken(false);
            self.tar = 0;
            self.epc = pc;
        }

        self.cause.set_coprocessor_number(coprocessor_number as u32);

        // Set the exception code in Cause
        self.cause.set_exception_code(cause.clone());

        // Determine and return the target address for the exception handler
        if self.status.boot_exception_vectors() {
            0xbfc0_0180
        } else {
            0x8000_0080
        }
    }

    /// Checks if bit #16 of the Status Register is set, which indicates that
    /// the cache should be made accessible to the CPU as the first 4KB of
    /// memory.
    pub fn isolate_cache(&self) -> bool {
        self.status.isolate_cache()
    }
}
