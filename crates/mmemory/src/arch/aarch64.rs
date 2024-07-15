use core::{ptr::slice_from_raw_parts, sync::atomic::AtomicU64};

use aarch64_cpu::registers::{CurrentEL, TTBR1_EL1};
use log::debug;
use tock_registers::interfaces::{Readable, Writeable};

use super::ArchCommon;
use crate::{BootState, MemRegionFlags, PhysAddr, VirtAddr};

pub struct Arch {
    current_t1: AtomicU64,
}

impl ArchCommon for Arch {
    fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
        PhysAddr::from(0)
    }

    fn init(&self, boot_state: impl BootState) {
        let reg: CurrentEL::EL::Value = CurrentEL.read_as_enum(CurrentEL::EL).unwrap();
        let el = match reg {
            CurrentEL::EL::Value::EL0 => "EL0",
            CurrentEL::EL::Value::EL1 => "EL1",
            CurrentEL::EL::Value::EL2 => "EL2",
            CurrentEL::EL::Value::EL3 => "EL3",
        };
        debug!("EL: {}", el);
        let kernel_init_size = 0x1000;


        let regions = BootState::memory_regions();
        for region in regions {
            if region.flags.contains(MemRegionFlags::FREE) {
                
                break;
            }
        }

        self.print_page();
    }
}

impl Arch {
    pub const fn new() -> Self {
        Self {
            current_t1: AtomicU64::new(0),
        }
    }

    fn print_page(&self) {}

    fn map(&self) {}
}
