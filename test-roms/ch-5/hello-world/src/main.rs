#![no_std]
#![no_main]

use psx;

#[unsafe(no_mangle)]
fn main() {
    psx::early_puts("Hello, World!\n");
    
    loop {}
}
