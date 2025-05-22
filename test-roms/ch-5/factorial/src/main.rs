#![no_std]
#![no_main]

use psx;

fn factorial(n: u32) -> u32 {
    if n == 0 {
        1
    } else {
        n * factorial(n - 1)
    }
}

#[unsafe(no_mangle)]
fn main() {
    psx::early_puts("Factorial of 10 is: ");
    let result = factorial(10);
    psx::early_putn(result);
    psx::early_putchar('\n');    

    loop {}
}
