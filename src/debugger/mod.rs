mod commands;

use crate::{bus::AccessSize, cpu::Cpu};
use clap::Parser;
use rustyline::{DefaultEditor, error::ReadlineError};
use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
};

use commands::{BreakpointCommand, DebuggerArgs, DebuggerCommand, Registers};

#[derive(Debug)]
pub struct Debugger {
    /// Steps until the debugger is entered.
    pub steps: usize,

    /// And instance of the disassembler, with its settings
    disasm: psdisasm::Disassembler,

    /// Rustyline instance for command line input, with no special configuration.
    editor: DefaultEditor,

    /// The addresses where the debugger will break the execution
    exec_breakpoints: HashMap<u32, Breakpoint>,

    /// The addresses where the debugger will break on memory read/write
    read_breakpoints: HashMap<u32, Breakpoint>,

    /// The addresses where the debugger will break on memory write
    write_breakpoints: HashMap<u32, Breakpoint>,

    /// Index of the next breakpoint to be added
    pub breakpoint_index: usize,

    pub triggered: Arc<AtomicBool>,

    last_cpu_regs: [u32; 32],
    last_gte_regs: [u32; 64],
}

pub enum BreakReason {
    /// The CPU was stepped
    Step,
    /// The user pressed Ctrl-C
    CtrlC,
    /// The PC hit a breakpoint
    PCBreakpoint(u32, bool),
    /// A memory read breakpoint was hit
    ReadBreakpoint(u32, bool),
    /// A memory write breakpoint was hit
    WriteBreakpoint(u32, bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Breakpoint {
    /// Whether this is a temporary breakpoint
    temporary: bool,

    /// Index of the breakpoint in the list
    index: usize,
}

const REGISTERS: [&str; 32] = [
    "$zero", "$at", "$v0", "$v1", "$a0", "$a1", "$a2", "$a3", "$t0", "$t1",
    "$t2", "$t3", "$t4", "$t5", "$t6", "$t7", "$s0", "$s1", "$s2", "$s3",
    "$s4", "$s5", "$s6", "$s7", "$t8", "$t9", "$k0", "$k1", "$gp", "$sp",
    "$fp", "$ra",
];

const HISTORY_FILE: &str = ".dbg_history";

impl Debugger {
    /// Create a new debugger instance
    pub fn new() -> Self {
        let mut editor = DefaultEditor::new().unwrap();
        let _ = editor.load_history(HISTORY_FILE);

        Debugger {
            steps: 0,
            disasm: psdisasm::Disassembler::default(),
            editor,
            exec_breakpoints: HashMap::new(),
            read_breakpoints: HashMap::new(),
            write_breakpoints: HashMap::new(),
            breakpoint_index: 0,
            triggered: Arc::new(AtomicBool::new(false)),
            last_cpu_regs: [0; 32],
            last_gte_regs: [0; 64],
        }
    }

    /// Enter the debugger
    pub fn enter(&mut self, cpu: &mut Cpu, reason: BreakReason) -> bool {
        match reason {
            BreakReason::Step => {}
            BreakReason::CtrlC => {
                // If the reason is a Ctrl-C, we should print a message
                println!("\nCtrl-C pressed, stopping execution...");
            }
            BreakReason::PCBreakpoint(addr, tmp) => {
                if !tmp {
                    println!("Hit breakpoint at address {:08x}", addr);
                }
            }
            BreakReason::ReadBreakpoint(addr, tmp) => {
                if !tmp {
                    println!(
                        "Hit memory read breakpoint at address {:08x}",
                        addr
                    );
                }
            }
            BreakReason::WriteBreakpoint(addr, tmp) => {
                if !tmp {
                    println!(
                        "Hit memory write breakpoint at address {:08x}",
                        addr
                    );
                }
            }
        }

        // Present the current instruction
        let ins = cpu.read_memory(cpu.pc, AccessSize::Word).unwrap();
        let is_branch_delay_slot = cpu.branch_target.is_some();

        println!(
            "[{:08x}] {}  {}",
            cpu.pc,
            if is_branch_delay_slot { "D" } else { " " },
            self.disasm.disassemble_with_context(
                ins,
                cpu.pc.wrapping_add(4),
                &cpu.registers
            )
        );

        loop {
            // Read a command from the user. Return true if this is None
            // (e.g. the user pressed Ctrl-C)
            let Some(line) = self.read_line() else {
                return true;
            };

            if line == "" {
                // Empty line is an alias for "step 1"
                self.steps = 1;
                break;
            }

            // Take the first word as the command
            let parts = line.split_whitespace();

            match DebuggerArgs::try_parse_from(parts) {
                Ok(cli) => {
                    match self.handle_command(cli.command, cpu) {
                        None => {
                            // Command was handled, continue executing
                            break;
                        }
                        Some(false) => {
                            // Command was handled, stay in the debugger
                            continue;
                        }
                        Some(true) => {
                            // Command was handled, quit the debugger
                            return true;
                        }
                    }
                }
                Err(e) => {
                    // If the command is not recognized, print the help message
                    e.print().unwrap();
                    continue;
                }
            }
        }

        self.last_cpu_regs.copy_from_slice(&cpu.registers);
        self.last_gte_regs.copy_from_slice(&cpu.gte.all_regs());

        return false;
    }

    fn handle_command(
        &mut self,
        command: DebuggerCommand,
        cpu: &mut Cpu,
    ) -> Option<bool> {
        match command {
            DebuggerCommand::Quit => {
                // Quit the debugger
                return Some(true);
            }
            DebuggerCommand::Step { steps } => {
                // Step the CPU by the given number of steps
                self.steps = steps;
                return None;
            }
            DebuggerCommand::Next => {
                // Step the CPU by one instruction, but don't enter functions
                // For all instructions, set self.steps to 1, except for JAL, JALR, BLTZAL, BGEZAL
                // For those, we set steps to 0 (free run), and a temporary breakpoint
                // at the instruction after the branch delay slot.

                let ins = cpu.read_memory(cpu.pc, AccessSize::Word).unwrap();
                let opcode = ins >> 26;
                let call = if opcode == 3 {
                    // JAL
                    true
                } else if opcode == 0 {
                    let funct = ins & 0x3F;
                    funct == 9
                } else if opcode == 1 {
                    let rt = ins >> 16 & 0x1f;
                    let link = rt & 0x10 == 0x10; // BLTZAL, BGEZAL
                    link
                } else {
                    false
                };

                if call {
                    // Set the steps to 0 (free run) and set a temporary breakpoint
                    // at the instruction after the branch delay slot.
                    self.steps = 0;
                    let next_pc = cpu.pc.wrapping_add(8);
                    self.exec_breakpoints.insert(
                        next_pc,
                        Breakpoint {
                            temporary: true,
                            index: 0,
                        },
                    );
                } else {
                    // Step by one instruction
                    self.steps = 1;
                }

                return None;
            }
            DebuggerCommand::Continue => {
                // Continue execution until the next breakpoint or step
                self.steps = 0;
                return None;
            }
            DebuggerCommand::Until { address } => {
                // Set a temporary breakpoint at the given address
                self.exec_breakpoints.insert(
                    address,
                    Breakpoint {
                        temporary: true,
                        index: 0,
                    },
                );

                // Set the PC to the given address
                self.steps = 0;

                return None;
            }
            DebuggerCommand::Registers { module } => {
                // Show the registers
                match module {
                    Registers::All => {
                        self.print_registers(cpu);
                        self.print_cop0_registers(cpu);
                        self.print_gte_registers(cpu);
                    }
                    Registers::Cpu => {
                        self.print_registers(cpu);
                    }
                    Registers::Gte => {
                        self.print_gte_registers(cpu);
                    }
                    Registers::Cop0 => {
                        self.print_cop0_registers(cpu);
                    }
                }
            }
            DebuggerCommand::Breakpoint { command } => {
                match command {
                    BreakpointCommand::Add {
                        address,
                        read,
                        write,
                        temporary,
                    } => {
                        // Add a breakpoint at the given address
                        if read && write {
                            println!(
                                "Cannot set a breakpoint for both read and write at the same time."
                            );
                            return Some(false);
                        }

                        let bp = Breakpoint {
                            temporary,
                            index: if temporary {
                                self.breakpoint_index += 1;
                                self.breakpoint_index - 1
                            } else {
                                0
                            },
                        };

                        if read {
                            // Add a read breakpoint
                            self.read_breakpoints.insert(address, bp);
                            println!(
                                "Read breakpoint added at {:08x}",
                                address
                            );
                        } else if write {
                            // Add a write breakpoint
                            self.write_breakpoints.insert(address, bp);
                            println!(
                                "Write breakpoint added at {:08x}",
                                address
                            );
                        } else {
                            // Add an execute breakpoint
                            self.exec_breakpoints.insert(address, bp);
                            println!(
                                "Execute breakpoint added at {:08x}",
                                address
                            );
                        }
                    }
                    BreakpointCommand::List => {
                        // List all breakpoints from the 3 sets.
                        println!("Breakpoints:");
                        for (address, bp) in &self.exec_breakpoints {
                            println!(
                                "[{}] Execute breakpoint at {address:08x}{}",
                                bp.index,
                                if bp.temporary { " (temporary)" } else { "" }
                            );
                        }
                        for (address, bp) in &self.read_breakpoints {
                            println!(
                                "[{}] Read breakpoint at {address:08x}{}",
                                bp.index,
                                if bp.temporary { " (temporary)" } else { "" }
                            );
                        }
                        for (address, bp) in &self.write_breakpoints {
                            println!(
                                "[{}] Write breakpoint at {address:08x}{}",
                                bp.index,
                                if bp.temporary { " (temporary)" } else { "" }
                            );
                        }
                    }
                    BreakpointCommand::Remove { index } => {
                        // Remove a breakpoint at the given index
                        let mut found_addr = usize::MAX;
                        for (address, bp) in self.exec_breakpoints.iter() {
                            if bp.index == index {
                                found_addr = *address as usize;
                                break;
                            }
                        }

                        if found_addr != usize::MAX {
                            // Remove the execute breakpoint
                            self.exec_breakpoints.remove(&(found_addr as u32));
                            println!(
                                "Execute breakpoint removed at index {}",
                                index
                            );
                            return Some(false);
                        }

                        for (address, bp) in self.read_breakpoints.iter() {
                            if bp.index == index {
                                found_addr = *address as usize;
                                break;
                            }
                        }

                        if found_addr != usize::MAX {
                            // Remove the read breakpoint
                            self.read_breakpoints.remove(&(found_addr as u32));
                            println!(
                                "Read breakpoint removed at index {}",
                                index
                            );
                            return Some(false);
                        }

                        for (address, bp) in self.write_breakpoints.iter() {
                            if bp.index == index {
                                found_addr = *address as usize;
                                break;
                            }
                        }

                        if found_addr != usize::MAX {
                            // Remove the write breakpoint
                            self.write_breakpoints.remove(&(found_addr as u32));
                            println!(
                                "Write breakpoint removed at index {}",
                                index
                            );
                            return Some(false);
                        }

                        // If we reach here, no breakpoint was found
                        println!("No breakpoint found with index {}", index);
                    }
                }
            }
            DebuggerCommand::Jump { address } => {
                // Set the PC to the given address
                cpu.pc = address;
                println!("Jumping to {:08x}", address);
            }
            DebuggerCommand::Disasm { address, count } => {
                let address = address.unwrap_or(cpu.pc);

                // Disassemble the memory at the given address
                for i in 0..count {
                    let addr = address + (i as u32 * 4);
                    match cpu.read_memory(addr, AccessSize::Word) {
                        Ok(ins) => {
                            println!(
                                "{:08x}: {}",
                                addr,
                                self.disasm
                                    .disassemble(ins, addr.wrapping_add(4),)
                            );
                        }
                        Err(_) => println!(
                            "Error reading memory at address {:08x}",
                            addr
                        ),
                    }
                }
            }
            DebuggerCommand::ReadMemory {
                mut address,
                count,
                format,
                size,
            } => {
                let size: AccessSize =
                    size.try_into().unwrap_or(AccessSize::Word);

                // Read memory at the given address
                for _ in 0..count {
                    match cpu.read_memory(address, size) {
                        Ok(value) => match format.as_str() {
                            "hex" => println!(
                                "{address:08x}: {value:0nibbles$x}",
                                nibbles = size as usize * 2,
                            ),
                            "dec" => {
                                println!("{address:08x}: {}", value as u32)
                            }
                            _ => println!("Unknown format: {}", format),
                        },
                        Err(_) => println!(
                            "Error reading memory at address {address:08x}"
                        ),
                    }

                    // Increment the address by the size of the read
                    address = address.wrapping_add(size as u32);
                }
            }
            DebuggerCommand::WriteMemory {
                address,
                value,
                size,
            } => {
                println!(
                    "Writing memory at address {:08x} with value {:08x} and size {}",
                    address, value, size
                );

                let size: AccessSize =
                    size.try_into().unwrap_or(AccessSize::Word);

                // Write memory at the given address
                match cpu.write_memory(address, value, size) {
                    Ok(_) => println!(
                        "Wrote {:08x} to address {:08x}",
                        value, address
                    ),
                    Err(_) => println!(
                        "Error writing memory at address {:08x}",
                        address
                    ),
                }
            }
            DebuggerCommand::DumpRam { file } => {
                const RAM_SIZE: usize = 2 * 1024 * 1024; // 2MB of RAM

                // Dump the contents of the RAM to a file
                let mut contents: Vec<u8> = Vec::with_capacity(RAM_SIZE);
                let ram = &cpu.bus.ram;

                for i in 0..RAM_SIZE {
                    let byte = ram.read(i as u32, AccessSize::Byte) as u8;
                    contents.push(byte);
                }

                if let Err(e) = std::fs::write(&file, contents) {
                    println!("Error dumping RAM: {}", e);
                } else {
                    println!("RAM dumped to {}", file);
                }
            }
            DebuggerCommand::DumpVram { file, raw } => {
                const VRAM_SIZE: usize = 1024 * 512; // 512k * 16-bit entries
                let data = &cpu.bus.gpu.vram;

                if raw {
                    // Dump the contents of the VRAM to a file
                    let mut contents: Vec<u8> = Vec::with_capacity(VRAM_SIZE);

                    for i in 0..VRAM_SIZE {
                        let half = data[i];
                        contents.push(half as u8);
                        contents.push((half >> 8) as u8);
                    }

                    if let Err(e) = std::fs::write(&file, contents) {
                        println!("Error dumping RAM: {}", e);
                    } else {
                        println!("RAM dumped to {}", file);
                    }
                } else {
                    let mut buf = Vec::with_capacity(VRAM_SIZE * 3);
                    for &pixel in data {
                        let r = (pixel & 0x1F) << 3; // Blue
                        let g = ((pixel >> 5) & 0x1F) << 3; // Green
                        let b = ((pixel >> 10) & 0x1F) << 3; // Red

                        buf.push(r as u8);
                        buf.push(g as u8);
                        buf.push(b as u8);
                    }

                    // Create a PNG image from the VRAM data
                    image::save_buffer(
                        file,
                        &buf,
                        1024, // Width of the VRAM
                        512,  // Height of the VRAM
                        image::ColorType::Rgb8,
                    )
                    .unwrap_or_else(|e| {
                        println!("Error saving VRAM as PNG: {}", e);
                    });
                }
            }
        };

        Some(false)
    }

    /// Parses a string as a hexadecimal number, allowing for an optional "0x" prefix.
    fn parse_hex(string: &str) -> Result<u32, std::num::ParseIntError> {
        let string = string.strip_prefix("0x").unwrap_or(string);

        u32::from_str_radix(string, 16)
    }

    /// Read a line from the user
    pub fn read_line(&mut self) -> Option<String> {
        match self.editor.readline("> ") {
            Ok(line) => {
                // Add the line to the history
                let line = line.trim().to_string();

                // Add the line to the history
                let _ = self.editor.add_history_entry(&line);

                Some(line)
            }
            Err(ReadlineError::Interrupted) => None,
            Err(_) => {
                println!("Error reading line");
                Some(String::new())
            }
        }
    }

    /// Prints the contents of the registers
    pub fn print_registers(&self, cpu: &Cpu) {
        for (i, &value) in cpu.registers.iter().enumerate() {
            if value == self.last_cpu_regs[i] {
                // If the value is the same as the last time, print it normally
                print!("{:>5} -> {value:08x}  ", REGISTERS[i]);
            } else {
                // If the value is different, print it in green
                print!("{:>5} -> \x1b[32m{value:08x}\x1b[0m  ", REGISTERS[i]);
            }

            if i % 4 == 3 {
                println!();
            }
        }

        println!(
            "   pc -> {:08x}     hi -> {:08x}     lo -> {:08x}",
            cpu.pc, cpu.hi, cpu.lo
        );

        if let Some(load_delay) = &cpu.load_delay {
            println!(
                "Pending load: {} -> {:08x}",
                REGISTERS[load_delay.target], load_delay.value
            );
        }
    }

    pub fn print_cop0_registers(&self, cpu: &Cpu) {
        let cop = &cpu.cop0;

        let bpc = cop.bpc;
        let bda = cop.bda;
        let tar = cop.tar;
        let dcic = cop.dcic;
        let bad_vaddr = cop.bad_vaddr;
        let bdma = cop.bdma;
        let bpcm = cop.bpcm;
        let status = cop.status;
        let cause = cop.cause;
        let epc = cop.epc;

        println!("COP0 Registers:");

        print!("  Status: ");
        print!(
            "{}",
            if status.isolate_cache() {
                "BVE "
            } else {
                "bve "
            }
        );
        print!("{}", if status.isolate_cache() { "IC " } else { "ic " });

        print!("IM{:08b}  ", status.interrupt_mask());

        print!(
            "{}",
            if status.user_mode_old() {
                "KUo "
            } else {
                "kuo "
            }
        );
        print!(
            "{}",
            if status.interrupt_enable_old() {
                "IEo "
            } else {
                "ieo "
            }
        );

        print!(
            "{}",
            if status.user_mode_previous() {
                "KUp "
            } else {
                "kup "
            }
        );
        print!(
            "{}",
            if status.interrupt_enable_previous() {
                "IEp "
            } else {
                "iep "
            }
        );

        print!("{}", if status.user_mode() { "KU " } else { "ku " });
        print!(
            "{}",
            if status.interrupt_enable() {
                "IE "
            } else {
                "ie "
            }
        );
        println!("{:08x}", status.0);

        print!("  Cause: ");
        print!(" {:?} ", cause.exception_code());
        print!("IP{:08b}", cause.interrupt_pending());
        print!("{}", if cause.branch_delay() { " BD " } else { " bd " });
        println!("{:08x}", cause.0);

        println!("  EPC: {:08x}  BadVAddr: {:08x}", epc, bad_vaddr);

        println!("  bpc: {:08x}  bda: {:08x}  tar: {:08x}", bpc, bda, tar);
        println!(
            "  dcic: {:08x}  bdma: {:08x}  bpcm: {:08x}",
            dcic, bdma, bpcm
        );
    }

    const GTE_NAMES: [&str; 64] = [
        "V0_XY", "V0_Z", "V1_XY", "V1_Z", "V2_XY", "V2_Z", "RGBC", "OTZ",
        "IR0", "IR1", "IR2", "IR3", "SXY0", "SXY1", "SXY2", "SXYP", "SZ0",
        "SZ1", "SZ2", "SZ3", "RGB0", "RGB1", "RGB2", "RES1", "MAC0", "MAC1",
        "MAC2", "MAC3", "IRGB", "ORGB", "LZCS", "LZCR", "RT_0", "RT_1", "RT_2",
        "RT_3", "RT_4", "TRX", "TRY", "TRZ", "LLM_0", "LLM_1", "LLM_2",
        "LLM_3", "LLM_4", "RBK", "GBK", "BBK", "LCM_0", "LCM_1", "LCM_2",
        "LCM_3", "LCM_4", "RFC", "GFC", "BFC", "OFX", "OFY", "H", "DQA", "DQB",
        "ZSF3", "ZSF4", "FLAG",
    ];

    fn print_gte_registers(&self, cpu: &Cpu) {
        let gte = &cpu.gte;

        for i in 0..Self::GTE_NAMES.len() {
            let val = gte.read(i).unwrap();

            print!("{:>5} -> ", Self::GTE_NAMES[i]);
            if val == self.last_gte_regs[i] {
                // If the value is the same as the last time, print it normally
                print!("{:08x}  ", val);
            } else {
                // If the value is different, print it in green
                print!("\x1b[32m{:08x}\x1b[0m  ", val);
            }
            print!("{:08x}  ", val);

            if i % 4 == 3 {
                println!();
            }
        }
    }

    /// Checks if the given address is a breakpoint.
    pub fn has_breakpoint(&self, address: u32) -> bool {
        self.exec_breakpoints.contains_key(&address)
    }

    pub fn break_reason(&mut self, cpu: &Cpu) -> Option<BreakReason> {
        // Check if the CPU is in stepping mode
        if self.steps > 0 {
            // Decrement the step count
            self.steps -= 1;
            // If we have no steps left, we should break
            if self.steps == 0 {
                // If the steps are zero, we should break
                return Some(BreakReason::Step);
            }
        }

        if self.triggered.load(std::sync::atomic::Ordering::SeqCst) {
            // If the triggered flag is set, we should break
            self.triggered
                .store(false, std::sync::atomic::Ordering::SeqCst);
            return Some(BreakReason::CtrlC);
        }

        // Check if the current PC is a breakpoint
        let bp = self.has_breakpoint(cpu.pc);
        if bp {
            let temp = self.exec_breakpoints.get(&cpu.pc).unwrap().temporary;
            if temp {
                // If the value is true, it means this is a temporary breakpoint
                // Remove the breakpoint from the map
                self.exec_breakpoints.remove(&cpu.pc);
            }

            // If the PC is a breakpoint, we should break
            return Some(BreakReason::PCBreakpoint(cpu.pc, temp));
        } else {
            // Check against memory breakpoints
            let last_memory_access = cpu.last_memory_operation;
            if last_memory_access.0 == crate::cpu::AccessType::Read {
                let hit = self.read_breakpoints.get(&last_memory_access.1);
                if let Some(bp) = hit {
                    let tmp = bp.temporary;
                    if tmp {
                        // If the value is true, it means this is a temporary breakpoint
                        // Remove the breakpoint from the map
                        self.read_breakpoints.remove(&last_memory_access.1);
                    }
                    return Some(BreakReason::ReadBreakpoint(
                        last_memory_access.1,
                        tmp,
                    ));
                }
            } else if last_memory_access.0 == crate::cpu::AccessType::Write {
                let hit = self.write_breakpoints.get(&last_memory_access.1);
                if let Some(bp) = hit {
                    let tmp = bp.temporary;
                    if tmp {
                        // If the value is true, it means this is a temporary breakpoint
                        // Remove the breakpoint from the map
                        self.write_breakpoints.remove(&last_memory_access.1);
                    }
                    return Some(BreakReason::WriteBreakpoint(
                        last_memory_access.1,
                        tmp,
                    ));
                }
            }

            None
        }
    }
}

impl Drop for Debugger {
    fn drop(&mut self) {
        // Save the history to a file
        let _ = self.editor.save_history(HISTORY_FILE);
    }
}
