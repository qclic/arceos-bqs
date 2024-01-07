//! Common traits and types for xhci device drivers.

#![no_std]
#![feature(strict_provenance)]

use core::{alloc::Layout, num::NonZeroUsize};

use axhal::mem::phys_to_virt;
#[doc(no_inline)]
pub use driver_common::{BaseDriverOps, DeviceType};
use log::info;
use xhci::{
    accessor::Mapper,
    extended_capabilities::{self},
    ring::trb::event,
    ExtendedCapability, Registers,
};



#[cfg(feature = "vl805")]
pub mod vl805;






#[derive(Clone, Copy)]
struct MemoryMapper;

impl Mapper for MemoryMapper {
    unsafe fn map(&mut self, phys_base: usize, bytes: usize) -> NonZeroUsize {
        info!("mapping:{:x}", phys_base);
        return NonZeroUsize::new_unchecked(phys_to_virt(phys_base.into()).as_usize());
    }

    fn unmap(&mut self, virt_base: usize, bytes: usize) {}
}


