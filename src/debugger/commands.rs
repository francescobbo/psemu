use clap::{Parser, Subcommand, ValueEnum};

use crate::debugger::Debugger;

#[derive(Parser, Debug)]
#[command(multicall = true, name = "")]
pub struct DebuggerArgs {
    /// Debugger command
    #[command(subcommand)]
    pub command: DebuggerCommand,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum DebuggerCommand {
    /// Step the CPU by one or more instructions
    #[command(name = "step", alias = "s", about = "Step the CPU")]
    Step {
        /// Number of steps to take
        #[arg(default_value_t = 1)]
        steps: usize,
    },
    /// Step the CPU by one instruction, but doesn't enter functions
    #[command(
        name = "next",
        alias = "n",
        about = "Step the CPU without entering functions"
    )]
    Next,
    /// Continue execution until the next breakpoint or step
    #[command(name = "continue", alias = "c")]
    Continue,
    /// Continue execution until a specific address (or earlier breakpoint)
    #[command(
        name = "until",
        alias = "u",
        about = "Continue execution until a specific address"
    )]
    Until {
        /// Address to continue until
        #[arg(value_parser = Debugger::parse_hex)]
        address: u32,
    },
    /// Shows the contents of the CPU registers (or COP0, GTE)
    #[command(
        name = "registers",
        alias = "r",
        about = "Show the CPU registers"
    )]
    Registers {
        /// Choose which group of registers to show
        #[arg(default_value = "cpu")]
        module: Registers,
    },
    /// Manage breakpoints
    #[command(name = "breakpoint", alias = "b")]
    Breakpoint {
        #[command(subcommand)]
        command: BreakpointCommand,
    },
    /// Jump to a specific address, setting PC
    #[command(
        name = "jump",
        alias = "j",
        about = "Set PC to a specific address"
    )]
    Jump {
        /// Address to jump to
        #[arg(value_parser = Debugger::parse_hex)]
        address: u32,
    },
    /// Read one or more memory locations
    #[command(
        name = "mem-read",
        alias = "mr",
        about = "Read memory at a given address"
    )]
    ReadMemory {
        /// Address to read memory from
        #[arg(value_parser = Debugger::parse_hex)]
        address: u32,
        /// How many units to read
        #[arg(long = "count", short, default_value_t = 1)]
        count: usize,
        /// The format to show the memory in (e.g. hex, decimal)
        #[arg(long = "format", short, default_value = "hex")]
        format: String,
        /// Size of each unit to read
        #[arg(long = "size", short, default_value_t = 4)]
        size: usize,
    },
    /// Write a value to a specific memory address
    #[command(
        name = "write-memory",
        alias = "wm",
        about = "Write memory at a given address"
    )]
    WriteMemory {
        /// Address to write memory to
        #[arg(value_parser = Debugger::parse_hex)]
        address: u32,
        /// Value to write to the memory
        value: u32,
        /// Size of the value to write (in bytes)
        #[arg(long = "size", short, default_value_t = 4)]
        size: usize,
    },
    /// Disassemble memory at a given address
    #[command(name = "disasm", about = "Disassemble memory at a given address")]
    Disasm {
        /// Address to disassemble at
        #[arg(value_parser = Debugger::parse_hex)]
        address: Option<u32>,
        /// Number of instructions to disassemble
        #[arg(default_value_t = 10)]
        count: usize,
    },
    /// Dump the contents of the RAM to a file
    #[command(
        name = "dump-ram",
        alias = "d",
        about = "Dump the contents of the RAM to a file"
    )]
    DumpRam {
        /// File to dump the RAM to
        #[arg(value_name = "FILE")]
        file: String,
    },
    /// Dump the contents of the VRAM to a file
    #[command(
        name = "dump-vram",
        alias = "dv",
        about = "Dump the contents of the VRAM to a file"
    )]
    DumpVram {
        /// File to dump the VRAM to
        #[arg(value_name = "FILE")]
        file: String,

        /// Whether to dump the VRAM as raw data or as a PNG image
        #[arg(long, short, default_value_t = false)]
        raw: bool,
    },
    /// Quit the debugger
    #[command(name = "quit", alias = "q", about = "Quit the debugger")]
    Quit,
}

#[derive(Parser, ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Registers {
    /// Show all CPU registers
    All,
    /// Show only the CPU registers
    Cpu,
    /// Show only the GTE registers
    Gte,
    /// Show only the COP0 registers
    Cop0,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum BreakpointCommand {
    /// Add a breakpoint at the given address. If read or write are not specified,
    /// it will be an "execute" breakpoint.
    Add {
        /// Address to set the breakpoint at
        #[arg(value_parser = Debugger::parse_hex)]
        address: u32,
        /// Sets a "memory read" breakpoint
        #[arg(long, short)]
        read: bool,
        /// Sets a "memory write" breakpoint
        #[arg(long, short)]
        write: bool,
        /// Temprorary breakpoint, which will be removed after it is hit
        #[arg(long, short)]
        temporary: bool,
    },
    /// List all breakpoints
    List,
    /// Remove a breakpoint at the given address
    Remove {
        /// Address to remove the breakpoint from
        index: usize,
    },
}
