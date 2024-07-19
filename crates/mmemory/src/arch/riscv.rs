use super::ArchCommon;
use crate::{PhysAddr, VirtAddr, SIZE_4K};
use riscv::asm;
use riscv::register::{satp, sstatus, stvec};

pub struct Arch {}

impl ArchCommon for Arch {
    unsafe fn write_page_table_kernel(addr: PhysAddr) {
        satp::set(satp::Mode::Sv39, 0, addr.as_usize() >> 12);
        asm::sfence_vma_all();
    }

    fn vaddr_is_valid(vaddr: usize) -> bool {
        let top_bits = vaddr >> Self::PAGE_VA_MAX_BITS;
        top_bits == 0 || top_bits == 0xffff
    }
    /// Flushes the TLB.
    ///
    /// If `vaddr` is [`None`], flushes the entire TLB. Otherwise, flushes the TLB
    /// entry that maps the given virtual address.
    unsafe fn flush_tlb(vaddr: Option<VirtAddr>) {
        if let Some(vaddr) = vaddr {
            asm::sfence_vma(0, vaddr.as_usize())
        } else {
            asm::sfence_vma_all();
        }
    }
    
    const PAGE_LEVEL: usize = 3;

    const PAGE_SIZE: usize = SIZE_4K;

    const PAGE_PA_MAX_BITS: usize = 56;
    const PAGE_VA_MAX_BITS: usize = 39;
}
