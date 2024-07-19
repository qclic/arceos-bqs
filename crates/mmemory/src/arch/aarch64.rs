use aarch64_cpu::registers::TTBR1_EL1;
use core::arch::asm;
use memory_addr::PAGE_SIZE_4K;
use tock_registers::interfaces::Writeable;

use super::ArchCommon;
use crate::{PhysAddr, VirtAddr};

pub struct Arch {}

impl ArchCommon for Arch {
    unsafe fn write_page_table_kernel(addr: PhysAddr) {
        TTBR1_EL1.set(addr.as_usize() as _);
    }

    fn vaddr_is_valid(vaddr: usize) -> bool {
        let top_bits = vaddr >> Self::PAGE_VA_MAX_BITS;
        top_bits == 0 || top_bits == 0xffff
    }

    unsafe fn flush_tlb(vaddr: Option<VirtAddr>) {
        unsafe {
            if let Some(vaddr) = vaddr {
                asm!("tlbi vaae1is, {}; dsb sy; isb", in(reg) vaddr.as_usize())
            } else {
                // flush the entire TLB
                asm!("tlbi vmalle1; dsb sy; isb")
            }
        }
    }
    const PAGE_LEVEL: usize = 4;

    const PAGE_SIZE: usize = PAGE_SIZE_4K;

    const PAGE_PA_MAX_BITS: usize = 48;
    const PAGE_VA_MAX_BITS: usize = 48;
}

impl Arch {}
