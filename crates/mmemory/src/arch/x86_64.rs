use super::ArchCommon;
use crate::{BootState, PhysAddr, VirtAddr};
use memory_addr::PAGE_SIZE_4K;
use page_table_entry::aarch64::A64PTE;
use x86::{controlregs, msr, tlb};

pub struct Arch {}

impl ArchCommon for Arch {
    unsafe fn write_page_table_kernel(addr: PhysAddr) {
        unsafe { controlregs::cr3_write(addr.as_usize() as _) }
    }

    const PAGE_LEVEL: usize = 4;

    const PAGE_SIZE: usize = PAGE_SIZE_4K;

    const PAGE_PA_MAX_BITS: usize = 52;
    const PAGE_VA_MAX_BITS: usize = 48;

    unsafe fn flush_tlb(vaddr: Option<VirtAddr>) {
        if let Some(vaddr) = vaddr {
            unsafe { tlb::flush(vaddr.into()) }
        } else {
            unsafe { tlb::flush_all() }
        }
    }
}
impl Arch {
    pub const fn new() -> Self {
        Self {}
    }
}
