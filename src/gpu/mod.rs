pub struct Gpu {

}

impl Gpu {
    pub fn new() -> Self {
        Gpu {}
    }

    pub fn read(&self, address: u32) -> u32 {
        if address == 0x1f80_1814 {
            println!("[GPU] Read GPUSTAT");
            return 0x1c00_0000
        }

        println!("[GPU] Read operation at address {:#x}", address);
        0
    }

    pub fn write(&mut self, address: u32, value: u32) {
        println!("[GPU] Write operation at address {:#x} with value {:#x}", address, value);
    }
}