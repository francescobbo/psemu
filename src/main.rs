//[ mod-cpu
mod cpu;
//[ !omit
//[ mod-ram
mod ram;
//] mod-ram
//] !omit

use cpu::{AccessSize, Cpu};
//] mod-cpu

//[ !hello-world
fn main() {
    println!("Hello, world!");
}
//] !hello-world
