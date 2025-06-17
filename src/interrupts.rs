use crate::bus::AccessSize;

pub const INTERRUPTS_BASE: u32 = 0x1f80_1070;
pub const INTERRUPTS_END: u32 = 0x1f80_1077;

#[derive(Debug, Default)]
pub struct InterruptController {
    /// The interrupt status register
    pub i_stat: u16,

    /// The interrupt mask register
    pub i_mask: u16,
}

impl InterruptController {
    /// Creates a new interrupt controller with default values.
    pub fn new() -> Self {
        InterruptController::default()
    }

    /// Sets an IRQ for the given source.
    pub fn trigger_irq(&mut self, source: usize) {
        if source <= 10 {
            // println!(
            //     "[InterruptController] Triggering IRQ {} (I_STAT: {:#x}, I_MASK: {:#x})",
            //     source, self.i_stat, self.i_mask
            // );
            self.i_stat |= 1 << source;
        } else {
            unreachable!("Invalid IRQ source: {}", source);
        }
    }

    /// Returns true when the controller has pending and enabled interrupts.
    pub fn should_interrupt(&self) -> bool {
        if self.i_stat & self.i_mask & 4 != 0 {
            // println!("CDROM INT")
        }

        // Check if any interrupt is pending and enabled
        (self.i_stat & self.i_mask) != 0
    }

    pub fn read(&self, address: u32, _size: AccessSize) -> u32 {
        match address {
            0x1f801070 => self.i_stat as u32,
            0x1f801074 => self.i_mask as u32,
            _ => panic!(
                "[InterruptController] Invalid address for read: {:#x}",
                address
            ),
        }
    }

    pub fn write(&mut self, address: u32, value: u32, _size: AccessSize) {
        match address {
            // Writes to I_STAT are a masked write, meaning that bits written as
            // 1 are left unchanged, while bits written as 0 are cleared.
            0x1f801070 => self.i_stat &= value as u16,
            0x1f801074 => self.i_mask = value as u16,
            _ => panic!(
                "[InterruptController] Invalid address for write: {:#x}",
                address
            ),
        }
    }
}
