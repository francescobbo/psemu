use crate::{emulator::JoypadButton, interrupts::InterruptController};

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Idle,
    Transmitting,
    PendingTransmission { value: u8, ticks: isize },
    PendingAck { ticks: isize }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ActiveDevice {
    None,
    Controller,
    MemoryCard
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ControllerTransferState {
    Idle, Ready, IDMSB, ButtonsLSB, ButtonsMSB
}

pub struct Joy {
    button_state: u16,

    ctrl: u32,
    stat: u32,
    mode: u32,
    baud: u32,
    receive_buffer: u8,
    receive_buffer_full: bool,
    transmit_buffer: u8,
    transmit_buffer_full: bool,

    active_device: ActiveDevice,
    state: State,

    last_cycles: u64,

    controller_transfer_state: ControllerTransferState,
    irq_requested: bool
}

impl Joy {
    pub fn new() -> Self {
        Joy {
            button_state: 0xffff, // Default button state (all buttons released)

            ctrl: 0x0000_0000, // Default control register
            stat: 0x0000_0000, // Default status register
            mode: 0x0000_0000, // Default mode register
            baud: 0,
            receive_buffer: 0x00,
            receive_buffer_full: false,
            transmit_buffer: 0x00,
            transmit_buffer_full: false,

            active_device: ActiveDevice::None,
            state: State::Idle,
            last_cycles: 0,

            controller_transfer_state: ControllerTransferState::Idle,
            irq_requested: false
        }
    }

    pub fn press_button(&mut self, button: JoypadButton) {
        // println!("[JOY] Pressing button {button:?}");
        self.button_state &= !(1 << button as u16);
    }

    pub fn release_button(&mut self, button: JoypadButton) {
        // println!("[JOY] Releasing button {button:?}");
        self.button_state |= 1 << button as u16;
    }

    pub fn cycle(&mut self, cycles: u64, intc: &mut InterruptController) {
        let diff = (cycles - self.last_cycles) as isize;
        self.last_cycles = cycles;

        match self.state {
            State::PendingAck { ticks } => {
                let updated = ticks - diff;
                if updated < 0 {
                    self.perform_ack()
                } else {
                    self.state = State::PendingAck { ticks: updated };
                }
            }
            State::PendingTransmission { ticks, value } => {
                let updated = ticks - diff;
                if updated < 0 {
                    self.perform_transfer()
                } else {
                    self.state = State::PendingTransmission { ticks: updated, value };
                }
            }
            _ => {}
        }

        if self.stat & (1 << 9) != 0 && !self.irq_requested {
            self.irq_requested = true;
            intc.trigger_irq(7);
        }
    }

    pub fn read(&mut self, address: u32) -> u32 {
        // println!("READING JOY at {address:08x}");
        
        if address == 0x1f80_1040 {
            if matches!(self.state, State::PendingTransmission { .. }) {
                // println!("Performing early transfer due to data read");
                self.perform_transfer();
            }

            let value = if self.receive_buffer_full {
                self.receive_buffer
            } else {
                0xff
            } as u32;

            self.receive_buffer_full = false;
            self.update_status();

            value | (value << 8) | (value << 16) | (value << 24)
        } else if address == 0x1f80_1044 {
            if matches!(self.state, State::PendingTransmission { .. }) {
                self.perform_transfer();
            }

            let stat = self.stat;
            self.stat &= !(1<<7); // ???
            stat
        } else if address == 0x1f80_104a {
            // println!("[JOY] Read CTRL");
            self.ctrl
        } else if address == 0x1f80_104e {
            self.baud
        } else {
            panic!("[JOY] Unimplemented read at address {:#x}", address);
        }
    }

    pub fn write(&mut self, address: u32, value: u32) {
        // println!("WRITING JOY at {address:08x}: {value:08x}");
        
        if address == 0x1f80_1040 {
            if self.transmit_buffer_full {
                println!("[JOY] Buffer overrun");
            }

            self.transmit_buffer = value as u8;
            self.transmit_buffer_full = true;

            if self.stat & (1 << 10) != 0 {
                self.irq("TX");
            }

            if self.state == State::Idle && self.ctrl & 3 == 3 {
                self.begin_transfer();
            } else {
                println!("Data written but I'm in {:?} {:08x}", self.state, self.ctrl);
            }
        } else if address == 0x1f80_1048 {
            self.mode = value;
        } else if address == 0x1f80_104a {
            self.write_ctrl(value);
        } else if address == 0x1f80_104e {
            self.baud = value;
        } else {
            panic!("[JOY] Unimplemented write at address {:#x}", address);
        }
    }

    fn write_ctrl(&mut self, value: u32) {
        self.ctrl = value;

        if value & 0x40 != 0 {
            // Reset
            self.ctrl = 0;
            self.stat = 0;
            self.mode = 0;
            self.receive_buffer = 0;
            self.receive_buffer_full = false;
            self.transmit_buffer = 0;
            self.transmit_buffer_full = false;
            self.active_device = ActiveDevice::None;
            // self.queue.clear();
            self.state = State::Idle;
            self.update_status();
        }

        if value & 2 == 0 && value & 1 == 0 {
            self.state = State::Idle; // Idle state
        } else {
            if self.state == State::Idle && self.transmit_buffer_full && value & 3 == 3 {
                self.begin_transfer();
            }
        }
    }

    fn begin_transfer(&mut self) {
        let value = self.transmit_buffer;
        self.transmit_buffer_full = false;

        self.ctrl |= 4;

        // println!("Begun transmit of value {value}");

        self.state = State::PendingTransmission { value, ticks: (self.baud * 8) as isize }
    }

    fn update_status(&mut self) {
        if self.receive_buffer_full {
            self.stat |= 0x02; // Set the receive buffer full status bit
        } else {
            self.stat &= !0x02; // Clear the receive buffer full status bit
        }

        if !self.transmit_buffer_full && self.state != State::Transmitting {
            self.stat |= 0x04; // Set the transmit buffer empty status bit
        } else {
            self.stat &= !0x04; // Clear the transmit buffer empty status bit
        }

        if !self.transmit_buffer_full {
            self.stat |= 0x01; // Set the transmit buffer full status bit
        } else {
            self.stat &= !0x01; // Clear the transmit buffer full status bit
        }
    }

    fn perform_transfer(&mut self) {
        let value = if let State::PendingTransmission { value, .. } = self.state {
            value
        } else {
            panic!("Not in a pending transmission")
        };

        // 0 = left pad and memcard, 1 = right pad and memcard
        // let device_index = (self.ctrl >> 13) & 1;

        self.ctrl |= 4;

        // Unless otherwise specified, response is 0xff
        let mut response = 0xff;
        let mut acknowledged = false;

        match self.active_device {
            ActiveDevice::None => {
                // This is the first "send", we need to test if anyone will pick up
                (response, acknowledged) = self.controller_transfer(value);
                if !acknowledged {
                    (response, acknowledged) = self.memcard_transfer(value);
                    if !acknowledged {
                        // println!("[PAD] Wut? No one picks up")
                    } else {
                        self.active_device = ActiveDevice::MemoryCard;
                    }
                } else {
                    // println!("Controller picked up");
                    self.active_device = ActiveDevice::Controller;
                }
            }
            ActiveDevice::Controller => {
                (response, acknowledged) = self.controller_transfer(value);
            }
            ActiveDevice::MemoryCard => {
                (response, acknowledged) = self.memcard_transfer(value);
            }
        }

        self.receive_buffer = response;
        self.receive_buffer_full = true;

        if self.ctrl & (1 << 11) != 0 {
            // IRQ on receive
            self.irq("RX");
        }

        if !acknowledged {
            // Device did not acknowledge (nothing to reply)
            self.active_device = ActiveDevice::None;
            self.state = State::Idle;
        } else {
            // The device acknowledged, this may raise another interrupt
            self.state = State::PendingAck { ticks: 450 }
        }

        self.update_status();
    }

    fn perform_ack(&mut self) {
        self.stat |= 1 << 7;

        if self.ctrl & (1 << 12) != 0 {
            self.irq("ACK")
        }

        self.state = State::Idle;
        self.update_status();

        if self.transmit_buffer_full && self.ctrl & 3 == 3 {
            self.begin_transfer();
        }
    }

    fn irq(&mut self, reason: &'static str) {
        // println!("JOY REQUESTING IRQ due to {reason}");
        self.irq_requested = false;
        self.stat |= 1 << 9;
    }

    fn controller_transfer(&mut self, value: u8) -> (u8, bool) {
        match self.controller_transfer_state {
            ControllerTransferState::Idle => {
                if value == 1 {
                    self.controller_transfer_state = ControllerTransferState::Ready;
                    return (0xff, true);
                }

                return (0xff, false);
            }
            ControllerTransferState::Ready => {
                if value == 0x42 {
                    self.controller_transfer_state = ControllerTransferState::IDMSB;
                    // 0x41 = low byte of Controller Identifier
                    return (0x41, true);
                }

                return (0xff, false);
            }
            ControllerTransferState::IDMSB => {
                self.controller_transfer_state = ControllerTransferState::ButtonsLSB;
                // 0x51 = high byte of controller identifier
                return (0x5a, true)
            }
            ControllerTransferState::ButtonsLSB => {
                self.controller_transfer_state = ControllerTransferState::ButtonsMSB;
                return ((self.button_state & 0xff) as u8, true)
            }
            ControllerTransferState::ButtonsMSB => {
                self.controller_transfer_state = ControllerTransferState::Idle;
                return ((self.button_state >> 8) as u8, false)
            }
        }
    }

    fn memcard_transfer(&mut self, value: u8) -> (u8, bool) {
        (0, false)
    }
}
