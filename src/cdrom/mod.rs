use std::{
    cmp::Ordering,
    ops::{Add, AddAssign, Sub, SubAssign},
    str::FromStr,
    vec,
};

use crate::{bus::AccessSize, interrupts::InterruptController};

mod cuebin;
mod reader;

const BYTES_PER_SECTOR: usize = 2352;

type SectorBuffer = [u8; BYTES_PER_SECTOR];

#[derive(Debug)]
pub struct Cdrom {
    bank: u8,
    response: [u8; 16],
    response_read_index: usize,
    response_write_index: usize,
    parameters: [u8; 16],
    parameter_index: usize,
    has_response: bool,
    int_mask: u8,
    int_status: u8,
    command_state: CommandState,
    drive_state: DriveState,
    seek_location: Option<CdTime>,
    sector_buffer: Box<SectorBuffer>,

    disc: cuebin::CdBinFiles<std::fs::File>,
    cue: cuebin::CueSheet,
    data_fifo: DataFifo,

    raw_sectors: bool,
}

impl Cdrom {
    pub fn new() -> Self {
        let file = "cb/Crash Bandicoot 1.cue";
        let cdbin_file =
            cuebin::CdBinFiles::create(file, |f| std::fs::File::open(f));

        Cdrom {
            bank: 0,                 // Default bank value
            response: [0; 16],       // Initialize response buffer
            response_read_index: 0,  // Initialize read index
            response_write_index: 0, // Initialize write index
            parameters: [0; 16],     // Initialize parameters buffer
            parameter_index: 0,      // Initialize parameter index
            has_response: false,     // Initially no response
            int_mask: 0,             // Default interrupt mask
            int_status: 0,           // Default interrupt status

            command_state: CommandState::Idle, // Initialize command state
            drive_state: DriveState::Stopped,  // Start with the drive stopped
            seek_location: None,               // No seek location set initially
            sector_buffer: Box::new([0; BYTES_PER_SECTOR]), // Initialize sector buffer
            disc: cdbin_file.0, // Initialize the disc with the CUE/BIN files
            cue: cdbin_file.1,  // Initialize the CUE sheet
            data_fifo: DataFifo::new(), // Initialize the data FIFO

            raw_sectors: false, // Default to not using raw sectors
        }
    }

    fn write_result(&mut self, result: Vec<u8>) {
        if result.len() > self.response.len() {
            panic!("[CDROM] Result too large to fit in response buffer");
        }

        self.response_write_index = 0;

        // Clear the response buffer
        self.response.fill(0);

        // Copy the result into the response buffer
        for (i, &byte) in result.iter().enumerate() {
            if i < self.response.len() {
                self.response[i] = byte;
                self.response_write_index += 1;
            } else {
                break; // Prevent overflow
            }
        }

        self.has_response = true; // Indicate that we have a response
    }

    pub fn read(&mut self, address: u32, size: AccessSize) -> u32 {
        // println!(
        //     "[CDROM] Read from address {:#x} with size {:?}",
        //     address, size
        // );
        match address - 0x1f80_1800 {
            0 => {
                let receiving_command = matches!(
                    self.command_state,
                    CommandState::CommandQueued { .. }
                        | CommandState::ReceivingCommand { .. }
                );

                let rslrddy = (self.has_response as u8) << 5;
                let receiving_command = (receiving_command as u8) << 7;
                let prmempt =
                    if self.parameter_index == 0 { 1 << 3 } else { 0 };
                let prmwrdy = if self.parameter_index < self.parameters.len() {
                    1 << 4
                } else {
                    0
                };
                let data = (!self.data_fifo.fully_consumed() as u32) << 6;

                let val = self.bank as u32
                    | (1 << 3)
                    | rslrddy as u32
                    | receiving_command as u32
                    | prmempt as u32
                    | prmwrdy as u32
                    | data;

                // println!("[CDROM] Read HSTS: {:#x}", val);
                val
            }
            1 => {
                // Return the response byte at the current index
                let data = self.response[self.response_read_index];
                self.response_read_index += 1;
                if self.response_read_index >= self.response.len() {
                    self.response_read_index = 0; // Reset index if it exceeds the length
                }

                if self.response_write_index == self.response_read_index {
                    // If the read index catches up to the write index, reset it
                    self.has_response = false;
                }

                data as u32
            }
            2 => match size {
                AccessSize::Byte => self.get_sector_byte() as u32,
                AccessSize::HalfWord => {
                    let data = [self.get_sector_byte(), self.get_sector_byte()];

                    i16::from_le_bytes(data) as u32
                }
                AccessSize::Word => {
                    let data = [
                        self.get_sector_byte(),
                        self.get_sector_byte(),
                        self.get_sector_byte(),
                        self.get_sector_byte(),
                    ];

                    u32::from_le_bytes(data)
                }
            },
            3 => {
                let val = if self.bank == 0 || self.bank == 2 {
                    // Return the interrupt mask
                    self.int_mask as u32 | 0xe0
                } else {
                    // For other banks, return the interrupt status
                    self.int_status as u32 | 0xe0
                };

                // println!("[CDROM] Read HINT/HMSK: {:#x}", val);
                val
            }
            _ => unreachable!(
                "[CDROM] Unimplemented read at address {:#x}",
                address
            ),
        }
    }

    pub fn write(&mut self, address: u32, value: u32) {
        // println!(
        //     "[CDROM] Write to address {:#x} with value {:#x}",
        //     address, value
        // );

        match address - 0x1f80_1800 {
            0 => {
                self.bank = (value & 0x03) as u8; // Set the bank to the lower 2 bits
            }
            1 => match self.bank {
                0 => self.write_command(value as u8),
                _ => {
                    println!(
                        "[CDROM] Write to 1 at bank {}: {:#x}",
                        self.bank, value
                    );
                }
            },
            2 => {
                match self.bank {
                    0 => {
                        println!("[CDROM] Write of parameter: {:#x}", value);

                        // Parameters write
                        if self.parameter_index < self.parameters.len() {
                            self.parameters[self.parameter_index] =
                                (value & 0xff) as u8; // Store the lower byte
                            self.parameter_index += 1;
                        } else {
                            println!(
                                "[CDROM] Parameter index out of bounds: {}",
                                self.parameter_index
                            );
                        }
                    }
                    1 => {
                        // Write the interrupt mask
                        self.int_mask = (value & 0x1f) as u8; // Mask to lower 5 bits
                    }
                    _ => {
                        println!(
                            "[CDROM] Write to 2 at bank {}: {:#x}",
                            self.bank, value
                        );
                        // For other banks, we might want to handle it differently
                        // For now, just print the value
                    }
                }
            }
            3 => {
                match self.bank {
                    1 => {
                        // Acknowledge the interrupts
                        self.int_status &= !value as u8; // Clear the bits in the interrupt status
                    }
                    _ => {
                        println!(
                            "[CDROM] Write to 3 at bank {}: {:#x}",
                            self.bank, value
                        );
                    }
                }
            }
            _ => unreachable!(
                "[CDROM] Unimplemented write at address {:#x}",
                address
            ),
        }
    }

    pub fn get_sector_byte(&mut self) -> u8 {
        self.data_fifo.pop()
    }

    pub fn clock(&mut self, intc: &mut InterruptController) {
        self.drive_state_machine();
        self.command_state_machine();

        if self.int_mask & self.int_status != 0 {
            // If any interrupt is masked and pending, we should trigger an interrupt
            // println!("[CDROM] Triggering interrupt with status: {:#x}", self.int_status);
            intc.trigger_irq(2);
        }
    }

    fn drive_state_machine(&mut self) {
        self.drive_state = match self.drive_state {
            DriveState::Stopped => DriveState::Stopped,
            DriveState::SpinningUp {
                cycles_remaining: 1,
                next: SpinUpNextState::Pause,
            } => DriveState::Paused {
                time: CdTime::ZERO,
                int2_queued: true,
            },
            DriveState::SpinningUp {
                cycles_remaining: 1,
                next: SpinUpNextState::Seek(time, seek_next),
            } => {
                let seek_cycles =
                    std::cmp::max(24, estimate_seek_cycles(CdTime::ZERO, time));
                DriveState::Seeking {
                    destination: time,
                    cycles_remaining: seek_cycles,
                    next: seek_next,
                }
            }
            DriveState::SpinningUp {
                cycles_remaining,
                next,
            } => DriveState::SpinningUp {
                cycles_remaining: cycles_remaining - 1,
                next,
            },
            DriveState::Seeking {
                destination,
                cycles_remaining: 1,
                next: SeekNextState::Pause,
            } => DriveState::Paused {
                time: destination,
                int2_queued: true,
            },
            DriveState::Seeking {
                destination,
                cycles_remaining: 1,
                next: SeekNextState::Read,
            } => DriveState::PreparingToRead {
                time: destination,
                cycles_remaining: 5 * 588,
            },
            DriveState::Seeking {
                destination,
                cycles_remaining: 1,
                next: SeekNextState::Play,
            } => DriveState::PreparingToPlay {
                time: destination,
                cycles_remaining: 5 * 588,
            },
            DriveState::Seeking {
                destination,
                cycles_remaining,
                next,
            } => DriveState::Seeking {
                destination,
                cycles_remaining: cycles_remaining - 1,
                next,
            },
            DriveState::PreparingToRead {
                time,
                cycles_remaining: 1,
            } => {
                // self.xa_adpcm.clear_buffers();
                self.read_data_sector(time)
            }
            DriveState::PreparingToRead {
                time,
                cycles_remaining,
            } => DriveState::PreparingToRead {
                time,
                cycles_remaining: cycles_remaining - 1,
            },
            DriveState::Reading(state) => self.progress_read_state(state),
            DriveState::PreparingToPlay {
                time,
                cycles_remaining: 1,
            } => {
                unimplemented!(
                    "[CDROM] Preparing to play audio at time: {:?}",
                    time
                );
                // self.read_audio_sector(PlayState::new(time), true)?
            }
            DriveState::PreparingToPlay {
                time,
                cycles_remaining,
            } => DriveState::PreparingToPlay {
                time,
                cycles_remaining: cycles_remaining - 1,
            },
            DriveState::Playing(state) => unimplemented!(), //self.progress_play_state(state)?,
            DriveState::Paused {
                time,
                mut int2_queued,
            } => {
                if int2_queued && !(self.int_status & 7 != 0) {
                    self.write_result(vec![self.stat()]);
                    self.emit_int2();
                    int2_queued = false;
                }

                DriveState::Paused { time, int2_queued }
            }
        };
    }

    fn command_state_machine(&mut self) {
        self.command_state = match self.command_state {
            CommandState::Idle => CommandState::Idle,
            CommandState::CommandQueued { command, cycles } => {
                if (self.int_status & 7) != 0 {
                    CommandState::ReceivingCommand {
                        command,
                        cycles_remaining: cycles,
                    }
                } else {
                    // The controller will not acccept a command if any interrupts are queued
                    CommandState::CommandQueued { command, cycles }
                }
            }
            CommandState::ReceivingCommand {
                command,
                cycles_remaining: 1,
            } => self.execute_command(command),
            CommandState::ReceivingCommand {
                command,
                cycles_remaining,
            } => CommandState::ReceivingCommand {
                command,
                cycles_remaining: cycles_remaining - 1,
            },
            CommandState::GeneratingSecondResponse {
                command,
                cycles_remaining: 1,
            } => {
                if !(self.int_status & 7 != 0) {
                    self.generate_second_response(command)
                } else {
                    // If an interrupt is pending, the controller waits until it is cleared
                    CommandState::GeneratingSecondResponse {
                        command,
                        cycles_remaining: 1,
                    }
                }
            }
            CommandState::GeneratingSecondResponse {
                command,
                cycles_remaining,
            } => CommandState::GeneratingSecondResponse {
                command,
                cycles_remaining: cycles_remaining - 1,
            },
        }
    }

    fn generate_second_response(&mut self, command: Command) -> CommandState {
        match command {
            Command::GetId => self.get_id_second_response(),
            Command::Init => self.init_second_response(),
            Command::Pause => {
                self.write_result(vec![self.stat()]);
                self.emit_int2();
                CommandState::Idle
            }
            Command::ReadToc => self.read_toc_second_response(),
            // Command::Stop => self.stop_second_response(),
            _ => unimplemented!(
                "state, command {command:?} not implemented for second response"
            ),
        }
    }

    pub(super) fn read_toc_second_response(&mut self) -> CommandState {
        self.write_result(vec![self.stat()]);
        self.emit_int2();
        CommandState::Idle
    }

    fn execute_command(&mut self, command: Command) -> CommandState {
        match command {
            Command::GetStat => {
                // Get status command
                println!("[CDROM] Executing GetStat command");
                self.write_result(vec![self.stat()]);
                self.emit_int3();
                CommandState::Idle
            }
            Command::Demute => {
                self.write_result(vec![self.stat()]);
                self.emit_int3();

                CommandState::Idle
            }
            Command::GetId => {
                self.write_result(vec![self.stat()]);
                self.emit_int3();

                CommandState::GeneratingSecondResponse {
                    command: Command::GetId,
                    cycles_remaining: 24,
                }
            }
            Command::ReadToc => {
                // Get TOC command
                self.write_result(vec![self.stat()]);
                self.emit_int3();

                CommandState::GeneratingSecondResponse {
                    command: Command::ReadToc,
                    cycles_remaining: 44,
                }
            }
            Command::SetFilter => {
                self.write_result(vec![self.stat()]);
                self.emit_int3();

                self.parameter_index = 0;

                CommandState::Idle
            }
            Command::SetMode => {
                let mode = self.parameters[0];
                self.parameter_index = 0; // Reset parameter index

                self.raw_sectors = mode & 0x20 != 0;

                self.write_result(vec![self.stat()]);
                self.emit_int3();

                CommandState::Idle
            }
            Command::SetLoc => self.execute_set_loc(),
            Command::Init => {
                // self.drive_mode = DriveMode::from(0x20);

                if !matches!(self.drive_state, DriveState::Stopped | DriveState::SpinningUp { .. }) {
                    self.drive_state =
                        DriveState::Paused { time: self.drive_state.current_time(), int2_queued: false };
                }

                self.write_result(vec![self.stat()]);
                self.emit_int3();

                match self.drive_state {
                    DriveState::Stopped => {
                        self.drive_state = DriveState::SpinningUp {
                            cycles_remaining: 22050,
                            next: SpinUpNextState::Pause,
                        };
                        CommandState::Idle
                    }
                    DriveState::SpinningUp { cycles_remaining, .. } => {
                        self.drive_state =
                            DriveState::SpinningUp { cycles_remaining, next: SpinUpNextState::Pause };
                        CommandState::Idle
                    }
                    _ => CommandState::GeneratingSecondResponse {
                        command: Command::Init,
                        cycles_remaining: 24,
                    },
                }
            }
            Command::SeekL => {
                self.write_result(vec![self.stat()]);
                self.emit_int3();

                let seek_location = self
                    .seek_location
                    .take()
                    .unwrap_or(self.drive_state.current_time());
                self.drive_state = determine_drive_state(
                    self.drive_state,
                    seek_location,
                    SeekNextState::Pause,
                );

                println!(
                    "Executed Seek command to {seek_location:?}, drive state is {:?}",
                    self.drive_state
                );
                CommandState::Idle
            }
            Command::GetLocP => {
                let absolute_time = self.drive_state.current_time();
                let track = self.cue.find_track_by_time(absolute_time);

                let (track_number, index, relative_time) =
                    track.map_or((0xAA, 0x00, CdTime::ZERO), |track| {
                        let track_number = binary_to_bcd(track.number);
                        let index = u8::from(absolute_time >= track.effective_start_time());
                        let relative_time = absolute_time.to_sector_number().saturating_sub(track.effective_start_time().to_sector_number());

                        (track_number, index, CdTime::from_sector_number(relative_time))
                    });

                self.write_result(vec![
                    track_number,
                    index,
                    binary_to_bcd(relative_time.minutes),
                    binary_to_bcd(relative_time.seconds),
                    binary_to_bcd(relative_time.frames),
                    binary_to_bcd(absolute_time.minutes),
                    binary_to_bcd(absolute_time.seconds),
                    binary_to_bcd(absolute_time.frames),
                ]);

                self.emit_int3();
            
                CommandState::Idle
            }
            Command::ReadN | Command::ReadS => self.execute_read(),
            Command::Pause => {
                self.write_result(vec![self.stat()]);
                self.emit_int3();

                self.drive_state = DriveState::Paused {
                    time: self.drive_state.current_time(),
                    int2_queued: false,
                };

                let cycles_till_second_response = 5 * 588; // 5 frames at 60Hz, each frame is 588 cycles
                CommandState::GeneratingSecondResponse {
                    command: Command::Pause,
                    cycles_remaining: cycles_till_second_response,
                }
            }
            Command::Test => {
                let param = self.parameters[0];
                self.parameter_index = 0; // Reset parameter index

                match param {
                    0x20 => {
                        // Test command with parameter 0x20
                        println!(
                            "[CDROM] Executing Test command with parameter 0x20"
                        );
                        self.write_result(vec![0x95, 0x07, 0x24, 0xc1]);
                        self.emit_int3();
                    }
                    _ => {
                        // Unrecognized parameter
                        unimplemented!(
                            "[CDROM] Unrecognized Test command parameter: {:#x}",
                            param
                        );
                    }
                }

                CommandState::Idle
            }
            _ => unimplemented!("[CDROM] Unimplemented command: {:?}", command),
        }
    }

    fn emit_int1(&mut self) {
        self.int_status |= 0x01;
    }

    fn emit_int2(&mut self) {
        self.int_status |= 0x02;
    }

    fn emit_int3(&mut self) {
        self.int_status |= 0x03;
    }

    fn write_command(&mut self, command_byte: u8) {
        let receive_cycles = 60; // 60 CD-ROM controller cycles (each 768 cpu cycles)

        let (command, cycles) = match command_byte {
            0x01 => (Command::GetStat, receive_cycles),
            0x02 => (Command::SetLoc, receive_cycles),
            0x03 => (Command::Play, receive_cycles),
            0x06 => (Command::ReadN, receive_cycles),
            0x07 => (Command::MotorOn, receive_cycles),
            0x08 => (Command::Stop, receive_cycles),
            0x09 => (Command::Pause, receive_cycles),
            0x0A => (Command::Init, receive_cycles),
            0x0B => (Command::Mute, receive_cycles),
            0x0C => (Command::Demute, receive_cycles),
            0x0D => (Command::SetFilter, receive_cycles),
            0x0E => (Command::SetMode, receive_cycles),
            0x10 => (Command::GetLocL, receive_cycles),
            0x11 => (Command::GetLocP, receive_cycles),
            0x13 => (Command::GetTN, receive_cycles),
            0x14 => (Command::GetTD, receive_cycles),
            0x15 => (Command::SeekL, receive_cycles),
            0x16 => (Command::SeekP, receive_cycles),
            0x19 => (Command::Test, receive_cycles),
            0x1A => (Command::GetId, receive_cycles),
            0x1B => (Command::ReadS, receive_cycles),
            0x1E => (Command::ReadToc, receive_cycles),
            _ => todo!("Command byte {command_byte:02X}"),
        };

        self.command_state = //if (self.int_status & 7) != 0 {
        //     println!("[CDROM] Command {command:?} queued, interrupts pending");

        //     CommandState::CommandQueued { command, cycles }
        // } else {
            CommandState::ReceivingCommand { command, cycles_remaining: cycles }
        //};
    }

    fn stat(&self) -> u8 {
        let motor_on = !matches!(
            self.drive_state,
            DriveState::Stopped | DriveState::SpinningUp { .. }
        );
        let seeking = matches!(self.drive_state, DriveState::Seeking { .. });
        let reading = matches!(
            self.drive_state,
            DriveState::PreparingToRead { .. } | DriveState::Reading { .. }
        );
        let playing = matches!(
            self.drive_state,
            DriveState::PreparingToPlay { .. } | DriveState::Playing { .. }
        );

        // let error = errors.0 | u8::from(self.shell_opened);

        // error |
        (u8::from(motor_on) << 1)
            | (u8::from(false) << 4) // should be shell_opened, but not implemented
            | (u8::from(reading) << 5)
            | (u8::from(seeking) << 6)
            | (u8::from(playing) << 7)
    }

    pub(super) fn get_id_second_response(&mut self) -> CommandState {
        // match &self.disc {
        // TODO don't hardcode region
        // Some(disc) => {
        let status = self.stat();
        // let mode_byte = match disc.cue().track(1).mode {
        //     TrackMode::Mode2 => 0x20,
        //     TrackMode::Mode1 | TrackMode::Audio => 0x00,
        // };

        self.write_result(vec![
            status, 0x00, 0x20, 0x00, b'S', b'C', b'E', b'E',
        ]);
        self.emit_int2();
        // None => {
        //     // "No disc" response
        //     self.int5(&[0x08, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        // }

        CommandState::Idle
    }

    pub(super) fn init_second_response(&mut self) -> CommandState {
        self.write_result(vec![self.stat()]);
        self.emit_int2();
        CommandState::Idle
    }

    pub(super) fn execute_set_loc(&mut self) -> CommandState {
        if self.parameter_index != 3 {
            panic!(
                "[CDROM] SetLoc command requires 4 parameters, got {}",
                self.parameter_index
            );
        }

        let minutes = self.parameters[0];
        let seconds = self.parameters[1];
        let frames = self.parameters[2];

        self.parameter_index = 0; // Reset parameter index

        // BCD to binary conversion
        let minutes = (minutes >> 4) * 10 + (minutes & 0x0F);
        let seconds = (seconds >> 4) * 10 + (seconds & 0x0F);
        let frames = (frames >> 4) * 10 + (frames & 0x0F);

        println!(
            "[CDROM] SetLoc command with parameters: {minutes:02}:{seconds:02}:{frames:02}"
        );

        let cdtime = CdTime::new(minutes, seconds, frames);
        self.seek_location = Some(cdtime);
        self.write_result(vec![self.stat()]);
        self.emit_int3();

        CommandState::Idle
    }

    pub(super) fn execute_read(&mut self) -> CommandState {
        self.write_result(vec![self.stat()]);
        self.emit_int3();

        let seek_location = self
            .seek_location
            .take()
            .unwrap_or(self.drive_state.current_time());
        if matches!(self.drive_state, DriveState::Reading(ReadState { time, .. }) if time == seek_location)
        {
            return CommandState::Idle;
        }

        self.drive_state = determine_drive_state(
            self.drive_state,
            seek_location,
            SeekNextState::Read,
        );

        CommandState::Idle
    }

    pub(super) fn read_data_sector(&mut self, time: CdTime) -> DriveState {
        self.read_sector_atime(time);

        let file = self.sector_buffer[16];
        let channel = self.sector_buffer[17];
        let submode = self.sector_buffer[18];
        // let is_real_time_audio = submode.bit(2) && submode.bit(6);

        let mut should_generate_int1 = true;
        // if self.drive_mode.adpcm_enabled
        //     && is_real_time_audio
        //     && (!self.drive_mode.adpcm_filter_enabled
        //         || (self.xa_adpcm.file == file && self.xa_adpcm.channel == channel))
        // {
        //     unimplemented!(
        //         "[CDROM] Real-time audio sector with CD-XA ADPCM enabled at {time:?}"
        //     );
        //     // // CD-XA ADPCM sector; send to ADPCM decoder instead of the data FIFO
        //     // should_generate_int1 = false;

        //     // log::debug!("Decoding CD-XA ADPCM sector at {time}");
        //     // self.xa_adpcm.decode_sector(self.sector_buffer.as_ref());
        // } else if self.drive_mode.adpcm_filter_enabled && is_real_time_audio {
        //     // The controller does not send sectors to the data FIFO if ADPCM filtering is enabled
        //     // and this is a real-time audio sector
        //     // should_generate_int1 = false;
        //     unimplemented!(
        //         "[CDROM] Real-time audio sector with ADPCM filtering enabled at {time:?}"
        //     );
        // }

        DriveState::Reading(ReadState {
            time: time + CdTime::new(0, 0, 1),
            int1_generated: !should_generate_int1,
            cycles_till_next_sector: 294,
        })
    }

    fn read_sector_atime(&mut self, time: CdTime) {
        let Some(track) = self.cue.find_track_by_time(time) else {
            // TODO INT4+pause at disc end
            todo!("Read to end of disc");
        };

        let track_number = track.number;
        let relative_time = time - track.start_time;

        self.disc.read_sector(
            track_number,
            relative_time.to_sector_number(),
            self.sector_buffer.as_mut(),
        );
    }

    pub(super) fn progress_read_state(
        &mut self,
        ReadState {
            time,
            mut int1_generated,
            cycles_till_next_sector,
        }: ReadState,
    ) -> DriveState {
        // if let Some((sample_l, sample_r)) = self.xa_adpcm.maybe_output_sample() {
        //     self.current_audio_sample = (sample_l, sample_r);
        // }

        if cycles_till_next_sector == 1 {
            return self.read_data_sector(time);
        }

        if !int1_generated
            && !(self.int_status & 7 != 0)
            && !matches!(
                self.command_state,
                CommandState::ReceivingCommand { .. }
            )
        {
            int1_generated = true;
            self.write_result(vec![self.stat()]);
            self.emit_int1();

            // TODO should the copy wait until software requests the data sector?
            if self.raw_sectors {
                self.data_fifo.copy_from_slice(&self.sector_buffer[12..2352]);
            } else {
                self.data_fifo
                    .copy_from_slice(&self.sector_buffer[24..24 + 2048]);
            }
        }

        DriveState::Reading(ReadState {
            time,
            int1_generated,
            cycles_till_next_sector: cycles_till_next_sector - 1,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum CommandState {
    Idle,
    CommandQueued {
        command: Command,
        cycles: u32,
    },
    ReceivingCommand {
        command: Command,
        cycles_remaining: u32,
    },
    GeneratingSecondResponse {
        command: Command,
        cycles_remaining: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Demute,
    GetId,
    GetLocL,
    GetLocP,
    GetStat,
    GetTD,
    GetTN,
    Init,
    MotorOn,
    Mute,
    Pause,
    Play,
    ReadN,
    ReadS,
    ReadToc,
    SeekL,
    SeekP,
    SetFilter,
    SetLoc,
    SetMode,
    Stop,
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DriveState {
    Stopped,
    SpinningUp {
        cycles_remaining: u32,
        next: SpinUpNextState,
    },
    Seeking {
        destination: CdTime,
        cycles_remaining: u32,
        next: SeekNextState,
    },
    PreparingToRead {
        time: CdTime,
        cycles_remaining: u32,
    },
    Reading(ReadState),
    PreparingToPlay {
        time: CdTime,
        cycles_remaining: u32,
    },
    Playing(PlayState),
    Paused {
        time: CdTime,
        int2_queued: bool,
    },
}

impl DriveState {
    fn current_time(self) -> CdTime {
        match self {
            Self::Stopped | Self::SpinningUp { .. } => CdTime::ZERO,
            Self::Paused { time, .. }
            | Self::PreparingToRead { time, .. }
            | Self::Reading(ReadState { time, .. })
            | Self::PreparingToPlay { time, .. }
            | Self::Playing(PlayState { time, .. })
            | Self::Seeking {
                destination: time, ..
            } => time,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CdTime {
    pub minutes: u8,
    pub seconds: u8,
    pub frames: u8,
}

impl CdTime {
    pub const ZERO: Self = Self {
        minutes: 0,
        seconds: 0,
        frames: 0,
    };
    pub const SECTOR_0_START: Self = Self {
        minutes: 0,
        seconds: 2,
        frames: 0,
    };
    pub const DISC_END: Self = Self {
        minutes: 60,
        seconds: 3,
        frames: 74,
    };

    pub const MAX_MINUTES: u8 = 80;
    pub const SECONDS_PER_MINUTE: u8 = 60;
    pub const FRAMES_PER_SECOND: u8 = 75;

    pub const MAX_SECTORS: u32 = 360000;

    pub fn to_sector_number(self) -> u32 {
        (u32::from(Self::SECONDS_PER_MINUTE) * u32::from(self.minutes)
            + u32::from(self.seconds))
            * u32::from(Self::FRAMES_PER_SECOND)
            + u32::from(self.frames)
    }

    pub fn new(minutes: u8, seconds: u8, frames: u8) -> Self {
        assert!(
            minutes < Self::MAX_MINUTES,
            "Minutes must be less than {}",
            Self::MAX_MINUTES
        );
        assert!(
            seconds < Self::SECONDS_PER_MINUTE,
            "Seconds must be less than {}",
            Self::SECONDS_PER_MINUTE
        );
        assert!(
            frames < Self::FRAMES_PER_SECOND,
            "Frames must be less than {}",
            Self::FRAMES_PER_SECOND
        );

        Self {
            minutes,
            seconds,
            frames,
        }
    }

    pub fn from_sector_number(sector_number: u32) -> Self {
        // All Sega CD sector numbers are less than 360,000 (80 minutes)
        assert!(
            sector_number < Self::MAX_SECTORS,
            "Invalid sector number: {sector_number}"
        );

        let frames = sector_number % u32::from(Self::FRAMES_PER_SECOND);
        let seconds = (sector_number / u32::from(Self::FRAMES_PER_SECOND))
            % u32::from(Self::SECONDS_PER_MINUTE);
        let minutes = sector_number
            / (u32::from(Self::FRAMES_PER_SECOND)
                * u32::from(Self::SECONDS_PER_MINUTE));

        Self::new(minutes as u8, seconds as u8, frames as u8)
    }
}

impl Add for CdTime {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let (frames, carried) =
            add(self.frames, rhs.frames, false, Self::FRAMES_PER_SECOND);
        let (seconds, carried) =
            add(self.seconds, rhs.seconds, carried, Self::SECONDS_PER_MINUTE);
        let (minutes, _) =
            add(self.minutes, rhs.minutes, carried, Self::MAX_MINUTES);

        Self {
            minutes,
            seconds,
            frames,
        }
    }
}

impl AddAssign for CdTime {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for CdTime {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let (frames, borrowed) =
            sub(self.frames, rhs.frames, false, Self::FRAMES_PER_SECOND);
        let (seconds, borrowed) = sub(
            self.seconds,
            rhs.seconds,
            borrowed,
            Self::SECONDS_PER_MINUTE,
        );
        let (minutes, _) =
            sub(self.minutes, rhs.minutes, borrowed, Self::MAX_MINUTES);

        Self {
            minutes,
            seconds,
            frames,
        }
    }
}

impl SubAssign for CdTime {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl PartialOrd for CdTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CdTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.minutes
            .cmp(&other.minutes)
            .then(self.seconds.cmp(&other.seconds))
            .then(self.frames.cmp(&other.frames))
    }
}

fn add(a: u8, b: u8, overflow: bool, base: u8) -> (u8, bool) {
    let sum = a + b + u8::from(overflow);
    (sum % base, sum >= base)
}

fn sub(a: u8, b: u8, overflow: bool, base: u8) -> (u8, bool) {
    let operand_r = b + u8::from(overflow);
    if a < operand_r {
        (base - (operand_r - a), true)
    } else {
        (a - operand_r, false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeekNextState {
    Pause,
    Read,
    Play,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpinUpNextState {
    Pause,
    Seek(CdTime, SeekNextState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadState {
    pub time: CdTime,
    pub int1_generated: bool,
    pub cycles_till_next_sector: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayState {
    pub time: CdTime,
    pub sample_idx: u16,
    pub sectors_till_report: u8,
    pub next_report_type: AudioReportType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioReportType {
    Absolute,
    Relative,
}

pub(super) fn estimate_seek_cycles(
    current: CdTime,
    destination: CdTime,
) -> u32 {
    if current == destination {
        return 1;
    }

    let diff = if current < destination {
        destination - current
    } else {
        current - destination
    };
    let diff_sectors = diff.to_sector_number();

    // Assume that it takes about a second to seek 60 minutes
    // TODO this is not accurate, but accurate seek timings are possibly not known?
    let sectors_per_cycle = 270000.0 / 44100.0;
    (f64::from(diff_sectors) / sectors_per_cycle).ceil() as u32
}

pub(super) fn determine_drive_state(
    drive_state: DriveState,
    destination: CdTime,
    next: SeekNextState,
) -> DriveState {
    match drive_state {
        DriveState::Stopped => DriveState::SpinningUp {
            cycles_remaining: 22_050,
            next: SpinUpNextState::Seek(destination, next),
        },
        DriveState::SpinningUp {
            cycles_remaining, ..
        } => DriveState::SpinningUp {
            cycles_remaining,
            next: SpinUpNextState::Seek(destination, next),
        },
        DriveState::Seeking {
            destination: time, ..
        }
        | DriveState::PreparingToRead { time, .. }
        | DriveState::Reading(ReadState { time, .. })
        | DriveState::PreparingToPlay { time, .. }
        | DriveState::Playing(PlayState { time, .. })
        | DriveState::Paused { time, .. } => {
            let seek_cycles =
                std::cmp::max(24, estimate_seek_cycles(time, destination));
            DriveState::Seeking {
                destination,
                cycles_remaining: seek_cycles,
                next,
            }
        }
    }
}

impl FromStr for CdTime {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        if bytes.len() != 8 {
            return Err(format!("Invalid time length: {}", bytes.len()));
        }

        if bytes[2] != b':' || bytes[5] != b':' {
            return Err(format!("Unexpected time format: {s}"));
        }

        let err_fn = |_err| format!("Invalid time string: {s}");
        let minutes: u8 = s[0..2].parse().map_err(err_fn)?;
        let seconds: u8 = s[3..5].parse().map_err(err_fn)?;
        let frames: u8 = s[6..8].parse().map_err(err_fn)?;

        Ok(CdTime {
            minutes,
            seconds,
            frames,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DataFifo {
    values: Box<[u8; 2352 as usize]>,
    idx: usize,
    len: usize,
}

impl DataFifo {
    pub fn new() -> Self {
        Self {
            values: Box::new([0; 2352]),
            idx: 0,
            len: 0,
        }
    }

    pub fn copy_from_slice(&mut self, slice: &[u8]) {
        self.values[..slice.len()].copy_from_slice(slice);
        self.idx = 0;
        self.len = slice.len();
    }

    pub fn pop(&mut self) -> u8 {
        // Data FIFO repeatedly returns the last value if all elements are popped
        if self.len == 0 {
            return 0;
        } else if self.idx == self.len {
            return self.values[self.len - 1];
        }

        let value = self.values[self.idx];
        self.idx += 1;
        value
    }

    pub fn fully_consumed(&self) -> bool {
        self.idx == self.len
    }
}

fn binary_to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
}