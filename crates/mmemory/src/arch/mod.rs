use crate::{BootState, PhysAddr, VirtAddr};

cfg_if::cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        mod x86_64;
        pub(crate) type Arch = x86_64::Arch;
        pub(crate) type PageEntry = page_table_entry::x86_64::X64PTE;
    } else if #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))] {
        mod riscv;
        pub(crate) type Arch = riscv::Arch;
        pub(crate) type PageEntry = page_table_entry::riscv::Rv64PTE;
    } else if #[cfg(target_arch = "aarch64")]{
        mod aarch64;
        pub(crate) type Arch = aarch64::Arch;
        pub(crate) type PageEntry = page_table_entry::aarch64::A64PTE;
    }
}

pub(crate) trait ArchCommon {
    const PAGE_LEVEL: usize;
    const PAGE_SIZE: usize;
    const PAGE_PA_MAX_BITS: usize;
    const PAGE_VA_MAX_BITS: usize;
    /// The maximum physical address.
    const PAGE_PA_MAX_ADDR: usize = (1 << Self::PAGE_PA_MAX_BITS) - 1;
    unsafe fn write_page_table_kernel(addr: PhysAddr);
    /// Flushes the TLB.
    ///
    /// If `vaddr` is [`None`], flushes the entire TLB. Otherwise, flushes the TLB
    /// entry that maps the given virtual address.
    unsafe fn flush_tlb(vaddr: Option<VirtAddr>);
    /// Whether a given physical address is valid.
    #[inline]
    fn paddr_is_valid(paddr: usize) -> bool {
        paddr <= Self::PAGE_PA_MAX_ADDR // default
    }

    /// Whether a given virtual address is valid.
    #[inline]
    fn vaddr_is_valid(vaddr: usize) -> bool {
        // default: top bits sign extended
        let top_mask = usize::MAX << (Self::PAGE_VA_MAX_BITS - 1);
        (vaddr & top_mask) == 0 || (vaddr & top_mask) == top_mask
    }
}
