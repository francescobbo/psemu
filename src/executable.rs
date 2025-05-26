use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
};

#[derive(Debug)]
pub struct Header {
    /// The signature of the file, which is always "PS-X EXE" in ASCII
    pub signature: [u8; 8],
    /// The value of the requested program counter at the start of execution
    pub pc: u32,
    /// The value of the requested GP (r28) register
    pub gp: u32,
    /// The location of the program in memory
    pub load_address: u32,
    /// The size of the program in bytes (does not include this header)
    pub file_size: u32,
    /// The address of a location in memory that needs to be zeroed
    pub bss_start: u32,
    /// The size of the memory region that needs to be zeroed
    pub bss_size: u32,
    /// Initial SP and FP (R29 and R30)
    pub sp_fp: u32,
}

impl Header {
    /// Reads and initializes PS-X EXE header from a file
    pub fn from_file(file: &mut File) -> io::Result<Self> {
        let signature = read_array(file)?;
        read_array::<8>(file)?;
        let pc = u32::from_le_bytes(read_array(file)?);
        let gp = u32::from_le_bytes(read_array(file)?);
        let load_address = u32::from_le_bytes(read_array(file)?);
        let file_size = u32::from_le_bytes(read_array(file)?);
        read_array::<8>(file)?;
        let bss_start = u32::from_le_bytes(read_array(file)?);
        let bss_size = u32::from_le_bytes(read_array(file)?);
        let sp_base = u32::from_le_bytes(read_array(file)?);
        let sp_offset = u32::from_le_bytes(read_array(file)?);

        Ok(Header {
            signature,
            pc,
            gp,
            load_address,
            file_size,
            bss_start,
            bss_size,
            sp_fp: sp_base.wrapping_add(sp_offset),
        })
    }
}

/// A loaded PS-X EXE file
#[derive(Debug)]
pub struct Executable {
    /// The header of the executable
    pub header: Header,

    /// The program code
    pub code: Vec<u8>,
}

impl Executable {
    /// Loads a PS-X EXE file from the given path
    pub fn load(path: &String) -> io::Result<Self> {
        let mut file = File::open(path)?;

        // A minimal check: the file must be at least 2KB long
        let file_length = file.metadata()?.len();
        if file_length < 2 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File is too small to be a PSX executable",
            ));
        }

        let header = Header::from_file(&mut file)?;

        // Check the signature
        if header.signature != *b"PS-X EXE" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid PS-X EXE signature",
            ));
        }

        // The program code starts at offset 0x800 in the file.
        // header.file_size is the size of this code segment.
        if file_length < (0x800 + header.file_size as u64) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File is smaller than specified by header",
            ));
        }

        // Move the file pointer to the start of the code segment
        file.seek(SeekFrom::Start(0x800))?;

        // Read the program code into a vector
        let mut code = vec![0u8; header.file_size as usize];
        file.read_exact(&mut code)?;

        Ok(Executable { header, code })
    }

    /// Prepares the Cpu and system state for execution of this executable
    pub fn load_into(&self, cpu: &mut crate::cpu::Cpu) {
        // Set the program counter to the entry point
        cpu.pc = self.header.pc;

        // Set the global pointer (R28)
        cpu.registers[28] = self.header.gp;

        // Set the stack pointer (R29) and frame pointer (R30)
        cpu.registers[29] = self.header.sp_fp;
        cpu.registers[30] = self.header.sp_fp;

        // Load the program into RAM
        let start_address = self.header.load_address;
        for (i, byte) in self.code.iter().enumerate() {
            cpu.write_memory(
                start_address + i as u32,
                *byte as u32,
                crate::bus::AccessSize::Byte,
            )
            .expect("Failed to write to memory");
        }

        // Zero out the BSS section
        let bss_start = self.header.bss_start;
        for i in bss_start..(bss_start + self.header.bss_size) {
            cpu.write_memory(i, 0, crate::bus::AccessSize::Byte)
                .expect("Failed to write to memory");
        }
    }
}

/// Helper to read a fixed-size array from a file
fn read_array<const N: usize>(r: &mut File) -> io::Result<[u8; N]> {
    let mut buf = [0u8; N];
    r.read_exact(&mut buf)?;
    Ok(buf)
}
