use core::{alloc::Layout, ptr::NonNull};

use allocator::{AllocResult, BaseAllocator as _, ByteAllocator as _};
use axalloc::global_allocator;
use axhal::mem::virt_to_phys;
use kspin::SpinNoIrq;
use log::{debug, error, trace};
use memory_addr::{align_up, VirtAddr, PAGE_SIZE_4K};
use os_dma::DMAInfo;
use page_table_entry::MappingFlags;

use crate::phys_to_bus;

cfg_if::cfg_if! {
    if #[cfg(feature = "slab")] {
        use allocator::SlabByteAllocator as DefaultByteAllocator;
    } else if #[cfg(feature = "buddy")] {
        use allocator::BuddyByteAllocator as DefaultByteAllocator;
    } else if #[cfg(feature = "tlsf")] {
        use allocator::TlsfByteAllocator as DefaultByteAllocator;
    }
}

pub(crate) static ALLOCATOR: SpinNoIrq<Option<Allocator>> = SpinNoIrq::new(None);

extern "Rust" {
    fn _table_remap(vaddr: VirtAddr, size: usize, flags: MappingFlags);
}

pub(crate) struct Allocator {
    alloc: DefaultByteAllocator,
}

impl Allocator {
    pub fn new() -> Self {
        Self {
            alloc: DefaultByteAllocator::new(),
        }
    }

    pub unsafe fn alloc_coherent(&mut self, layout: Layout) -> AllocResult<DMAInfo> {
        loop {
            if let Ok(data) = self.alloc.alloc(layout) {
                let cpu_addr = data.as_ptr() as usize;
                let paddr = virt_to_phys(VirtAddr::from(cpu_addr));
                let bus_addr = phys_to_bus(paddr);

                return Ok(DMAInfo {
                    cpu_addr,
                    bus_addr: bus_addr.as_u64(),
                });
            } else {
                let align = PAGE_SIZE_4K.max(layout.align());
                let expand_size = align_up(layout.size().max(layout.align() + 1), align);
                let num_pages = expand_size / PAGE_SIZE_4K;

                let vaddr = global_allocator().alloc_pages(num_pages, align)?;
                let want_flags = MappingFlags::READ | MappingFlags::WRITE | MappingFlags::UNCACHED;
                trace!("remap flag to {:?}", want_flags);
                _table_remap(VirtAddr::from(vaddr), expand_size, want_flags);

                debug!(
                    "expand coherent memory: [{:#x}, {:#x}) len: {expand_size:#X}",
                    vaddr,
                    vaddr + expand_size
                );
                self.alloc
                    .add_memory(vaddr, expand_size)
                    .inspect_err(|e| error!("add memory fail: {e:?}"))?;
            }
        }
    }

    pub unsafe fn dealloc_coherent(&mut self, dma: DMAInfo, layout: Layout) {
        self.alloc
            .dealloc(NonNull::new_unchecked(dma.cpu_addr as *mut u8), layout)
    }
}
