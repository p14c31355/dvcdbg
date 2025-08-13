#![no_std]
#![no_main]

use dvcdbg as dd;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> ! {
    loop {}
}
