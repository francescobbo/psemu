#![no_std]
#![no_main]

use psx;

#[unsafe(no_mangle)]
fn main() {
    let mut a = 0;
    let mut b = 1;

    while b < 1000 {
        let c = a + b;
        a = b;
        b = c;

        // Print the Fibonacci number
        psx::early_putn(c);
        psx::early_putchar('\n');
    }

    // Example of using a recursive function
    psx::early_puts("The 30th Fibonacci number is (recursive, will take some time):\n");
    let fib_30 = fibonacci_recursive(30);
    psx::early_putn(fib_30);
    psx::early_putchar('\n');

    // Example of using a dynamic programming approach
    psx::early_puts("The 30th Fibonacci number is (dynamic programming):\n");
    let fib_30_dynamic = fibonacci_dynamic(30);
    psx::early_putn(fib_30_dynamic);
    psx::early_putchar('\n');

    loop {}
}

fn fibonacci_recursive(n: u32) -> u32 {
    if n == 0 {
        0
    } else if n == 1 {
        1
    } else {
        fibonacci_recursive(n - 1) + fibonacci_recursive(n - 2)
    }
}

static mut FIB: [u32; 100] = [0; 100];

fn fibonacci_dynamic(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }

    unsafe {
        if FIB[n as usize] != 0 {
            return FIB[n as usize];
        }
        FIB[n as usize] = fibonacci_dynamic(n - 1) + fibonacci_dynamic(n - 2);
        FIB[n as usize]
    }
}
