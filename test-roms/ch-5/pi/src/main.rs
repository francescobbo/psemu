#![no_std]
#![no_main]

use psx;

#[unsafe(no_mangle)]
fn main() {
    let mut result = 0.0;
    let mut divisor = 1;
    let mut odd = true;

    // Loop until the divisor is greater than 100
    while divisor <= 1000000 {
        let value = 4.0 / divisor as f32;

        // If the divisor is odd, add the value to the result
        if odd {
            result += value;
        } else {
            // If the divisor is even, subtract the value from the result
            result -= value;
        }

        divisor += 2;

        // Toggle the odd flag
        odd = !odd;
    }

    // Print the result
    psx::early_putf(result, 5);
    psx::early_putchar('\n');
    
    loop {}
}
