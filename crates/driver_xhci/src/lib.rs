//! Common traits and types for xhci device drivers.

#![no_std]
#![feature(strict_provenance)]

use core::{
    alloc::{GlobalAlloc, Layout},
    num::NonZeroUsize,
};

use axalloc::GlobalAllocator;
use axhal::mem::{phys_to_virt, virt_to_phys, PhysAddr, VirtAddr};
#[doc(no_inline)]
pub use driver_common::{BaseDriverOps, DevError, DevResult, DeviceType};
use log::info;
use page_table_entry::{aarch64::A64PTE, GenericPTE, MappingFlags};
use xhci::{accessor::Mapper, Registers};

pub struct XhciController {
    pub controller: Option<Registers<MemoryMapper>>,
}

pub const VL805_VENDOR_ID: u16 = 0x1106;
pub const VL805_DEVICE_ID: u16 = 0x3483;
pub const VL805_MMIO_BASE: usize = 0x6_0000_0000;

pub mod register_operations_init_xhci;

/// The information of the graphics device.
#[derive(Debug, Clone, Copy)]
pub struct XhciInfo {}

#[derive(Clone)]
struct MemoryMapper {
    // addr_offset: usize,
}

impl Mapper for MemoryMapper {
    unsafe fn map(&mut self, phys_base: usize, bytes: usize) -> NonZeroUsize {
        // let virt_to_phys = virt_to_phys(phys_base.into());
        // let from = A64PTE(phys_base);

        // info!("mapping");
        // let pte: A64PTE =
        //     page_table::GenericPTE::new_page(virt_to_phys, MappingFlags::DEVICE, false);
        // // A64PTE::
        // info!("mapped");
        // let phys_to_virt = page_table::PagingIf::phys_to_virt(PhysAddr::from(phys_base));
        info!("mapping:{:x}", phys_base);

        // return NonZeroUsize::new_unchecked(phys_base + self.addr_offset);
        return NonZeroUsize::new_unchecked(phys_to_virt(phys_base.into()).as_usize());
        // let phys_to_virt = phys_to_virt(PhysAddr::from(phys_base >> 1 << 1));

        // return NonZeroUsize::new_unchecked(phys_to_virt(from).as_usize());
        // return NonZeroUsize::new_unchecked(phys_to_virt.as_usize());

        // let ret = NonZeroUsize::new_unchecked(phys_to_virt.as_usize());
        // info!("return:{:x},byte:{:x}", ret, bytes);
        // return ret;
    }

    fn unmap(&mut self, virt_base: usize, bytes: usize) {}
}

impl XhciController {
    pub fn init(
        pci_bar_address: usize,
        bar_size: usize,
        cap_offset_usize: usize,
    ) -> XhciController {
        // let config_enable = phys_to_virt(PhysAddr::from(0xFA000000));
        // let config_enable: usize = 0x6_0000_0000;
        // unsafe {
        //     info!("writing!");
        //     // while let stat = (*(config_enable.as_usize() as *const u16)) as u16 == 0x10 {
        //     while let stat = (*(config_enable as *const u16)) as u16 == 0x10 {
        //         *((add + 0x04) as *mut u16) = 326;
        //         info!("status:{}", stat);
        //     }
        //     info!("writed!");
        // }

        info!(
            "received address:{:x},offset:{:x},offseted:{:x}",
            pci_bar_address,
            cap_offset_usize,
            pci_bar_address + cap_offset_usize
        );
        XhciController {
            controller: Some(unsafe {
                xhci::Registers::new(
                    pci_bar_address,
                    MemoryMapper {
                        // addr_offset: cap_offset_usize,
                    },
                )
            }),
        }

        // controller: unsafe { xhci::Registers::new(0xfd500000, MemoryMapper {}) },
        // XhciController { controller: None }
    }
}

fn reset_xhci() {}

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
