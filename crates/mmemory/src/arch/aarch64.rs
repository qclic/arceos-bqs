use core::{arch::asm, mem::size_of, ptr::slice_from_raw_parts, sync::atomic::AtomicU64};

use aarch64_cpu::registers::{CurrentEL, TTBR1_EL1};
use log::debug;
use memory_addr::PAGE_SIZE_4K;
use page_table_entry::aarch64::A64PTE;
use tock_registers::interfaces::{Readable, Writeable};

use super::ArchCommon;
use crate::{BootState, PhysAddr, VirtAddr};

pub struct Arch {}

impl ArchCommon for Arch {
    fn init(&self, boot_state: impl BootState) {
        let reg: CurrentEL::EL::Value = CurrentEL.read_as_enum(CurrentEL::EL).unwrap();
        let el = match reg {
            CurrentEL::EL::Value::EL0 => "EL0",
            CurrentEL::EL::Value::EL1 => "EL1",
            CurrentEL::EL::Value::EL2 => "EL2",
            CurrentEL::EL::Value::EL3 => "EL3",
        };
        debug!("EL: {}", el);

        self.print_page();
    }

    fn write_page_table_kernel(addr: PhysAddr) {
        TTBR1_EL1.set(addr.as_usize() as _);
        Self::flush_tlb(None);
    }

    fn vaddr_is_valid(vaddr: usize) -> bool {
        let top_bits = vaddr >> Self::PAGE_VA_MAX_BITS;
        top_bits == 0 || top_bits == 0xffff
    }

    const PAGE_LEVEL: usize = 4;

    const PAGE_SIZE: usize = PAGE_SIZE_4K;

    const PAGE_PA_MAX_BITS: usize = 48;
    const PAGE_VA_MAX_BITS: usize = 48;
}

impl Arch {
    pub const fn new() -> Self {
        Self {}
    }

    fn print_page(&self) {}

    fn map(&self) {}

    /// Flushes the TLB.
    ///
    /// If `vaddr` is [`None`], flushes the entire TLB. Otherwise, flushes the TLB
    /// entry that maps the given virtual address.
    #[inline]
    pub fn flush_tlb(vaddr: Option<VirtAddr>) {
        unsafe {
            if let Some(vaddr) = vaddr {
                asm!("tlbi vaae1is, {}; dsb sy; isb", in(reg) vaddr.as_usize())
            } else {
                // flush the entire TLB
                asm!("tlbi vmalle1; dsb sy; isb")
            }
        }
    }
}
