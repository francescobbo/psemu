pub struct Joy {
    ctrl: u32,
    queue: Vec<u32>,
    n: bool,
}

impl Joy {
    pub fn new() -> Self {
        Joy {
            ctrl: 0x0000_0000, // Default control state
            queue: Vec::new(),
            n: false,
        }
    }

    pub fn read(&mut self, address: u32) -> u32 {
        if address == 0x1f80_1040 {
            println!("[JOY] Read DATA");
            if self.queue.len() > 0 {
                let value = self.queue[0];
                println!("[JOY] Returning value {value:#x} from queue");
                return value;
            } else {
                // If the queue is empty, return a default value
                println!("[JOY] Queue is empty, returning 0");
                return 0;
            }
        } else if address == 0x1f80_104a {
            // println!("[JOY] Read CTRL");
            return self.ctrl;
        } else {
            panic!("[JOY] Unimplemented read at address {:#x}", address);
        }
    }

    pub fn write(&mut self, address: u32, value: u32) {
        if address == 0x1f80_1040 {
            println!("[JOY] write DATA {value:#x}");

            if value == 0x42 {
                self.queue = vec![
                    0x41,
                    0x5a,
                    if self.n { 0 } else { 0xff },
                    if self.n { 0 } else { 0xff },
                ];
                self.n = !self.n; // Toggle n for the next write
            } else if self.queue.len() > 0 {
                self.queue.remove(0);
            }
        } else if address == 0x1f80_104a {
            // println!("[JOY] write CTRL {value:#x}");
            self.ctrl = value;
        } else {
            panic!("[JOY] Unimplemented read at address {:#x}", address);
        }
    }
}
