#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;
use os_dma::ArrayCoherent;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let v = ArrayCoherent::<u8>::zero(30, 4096).unwrap();
    drop(v);
    println!("Hello, world!");
}
