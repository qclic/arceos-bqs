//! Page table manipulation.

use crate::mem::{phys_to_virt, virt_to_phys, PhysAddr, VirtAddr};
use crate::{global_allocator, SIZE_4K};
use core::{alloc::Layout, ptr::NonNull};
use page_table_multiarch::PagingHandler;

/// Implementation of [`PagingHandler`], to provide physical memory manipulation to
/// the [page_table_multiarch] crate.
pub struct PagingHandlerImpl;

impl PagingHandler for PagingHandlerImpl {
    fn alloc_frame() -> Option<PhysAddr> {
        unsafe {
            let layout = Layout::from_size_align_unchecked(SIZE_4K, SIZE_4K);
            global_allocator()
                .alloc_nolock(layout)
                .inspect(|ptr| ptr.as_ptr().write_bytes(0, SIZE_4K))
                .map(|vaddr| virt_to_phys(VirtAddr::from(vaddr.as_ptr() as usize)))
                .ok()
        }
    }

    fn dealloc_frame(paddr: PhysAddr) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(SIZE_4K, SIZE_4K);
            global_allocator().dealloc_nolock(
                NonNull::new_unchecked(phys_to_virt(paddr).as_usize() as _),
                layout,
            )
        }
    }

    #[inline]
    fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
        phys_to_virt(paddr)
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        /// The architecture-specific page table.
        pub type PageTable = page_table_multiarch::x86_64::X64PageTable<PagingHandlerImpl>;
    } else if #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))] {
        /// The architecture-specific page table.
        pub type PageTable = page_table_multiarch::riscv::Sv39PageTable<PagingHandlerImpl>;
    } else if #[cfg(target_arch = "aarch64")]{
        /// The architecture-specific page table.
        pub type PageTable = page_table_multiarch::aarch64::A64PageTable<PagingHandlerImpl>;
    }
}
