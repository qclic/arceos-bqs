//! Common traits and types for xhci device drivers.

#![no_std]

use core::{alloc::Layout, num::NonZeroUsize};

use axhal::mem::{phys_to_virt, PhysAddr};
#[doc(no_inline)]
pub use driver_common::{BaseDriverOps, DevError, DevResult, DeviceType};
use log::info;
use page_table_entry::{aarch64::A64PTE, GenericPTE, MappingFlags};
use xhci::{accessor::Mapper, Registers};

pub struct XhciController {
    controller: Registers<MemoryMapper>,
}

pub const VL805_VENDOR_ID: u16 = 0x1106;
pub const VL805_DEVICE_ID: u16 = 0x3483;

/// The information of the graphics device.
#[derive(Debug, Clone, Copy)]
pub struct XhciInfo {}

#[derive(Clone)]
struct MemoryMapper;
impl Mapper for MemoryMapper {
    unsafe fn map(&mut self, phys_base: usize, bytes: usize) -> NonZeroUsize {
        // axalloc::global_allocator
        let from = PhysAddr::from(phys_base);
        // let from = A64PTE(phys_base);

        let pte: A64PTE = page_table::GenericPTE::new_page(from, MappingFlags::all(), false);
        // A64PTE::

        // return NonZeroUsize::new_unchecked(phys_to_virt(from).as_usize());
        return NonZeroUsize::new_unchecked(phys_to_virt(from).as_usize());
    }

    fn unmap(&mut self, virt_base: usize, bytes: usize) {
        unimplemented!()
    }
}

impl XhciController {
    pub fn init(add: u64) -> XhciController {
        info!("received address:{:x}", add);
        XhciController {
            controller: unsafe {
                let mapper = MemoryMapper {};
                xhci::Registers::new(add.try_into().unwrap(), mapper)
            },
        }
    }
}

/// Operations that require a graphics device driver to implement.
pub trait XhciDriverOps: BaseDriverOps {
    /// Get the display information.
    fn info(&self) -> XhciInfo;
}

impl BaseDriverOps for XhciController {
    fn device_name(&self) -> &str {
        //todo  unimplemented!();
        "xhci-controller"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::XHCI
    }
}

impl XhciDriverOps for XhciController {
    fn info(&self) -> XhciInfo {
        todo!()
    }
}
