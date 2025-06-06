//[ mod-ram
mod ram;
//] mod-ram
//[ !hello-world

//[ define-access-size
pub enum AccessSize {
    Byte,
    HalfWord,
    Word,
}
//] define-access-size

fn main() {
    println!("Hello, world!");
}
//] !hello-world
