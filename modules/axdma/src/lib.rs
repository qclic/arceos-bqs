//! [ArceOS](https://github.com/arceos-org/arceos) global dma allocator.
//!
//! be registered as the `os-dma`’s default allocator.

#![no_std]

extern crate alloc;
use core::alloc::Layout;

pub use ::allocator::AllocResult;
use memory_addr::PhysAddr;
pub use os_dma::{BusAddr, DMAInfo};
mod allocator;
use allocator::ALLOCATOR;

/// Converts a physical address to a bus address.
///
/// It assumes that there is a linear mapping with the offset
/// [axconfig::PHYS_BUS_OFFSET], that maps all the physical memory to the virtual
/// space at the address plus the offset. So we have
/// `baddr = paddr + PHYS_BUS_OFFSET`.
#[inline]
pub const fn phys_to_bus(paddr: PhysAddr) -> BusAddr {
    BusAddr::new((paddr.as_usize() + axconfig::PHYS_BUS_OFFSET) as u64)
}

/// Allocates `coherent` memory that meets Direct Memory Access (DMA) requirements.
///
/// This function allocates a block of memory through the global allocator. The memory pages must be contiguous, undivided, and have consistent read and write access.
///
/// - `layout`: The memory layout, which describes the size and alignment requirements of the requested memory.
///
/// Returns an [DMAInfo] structure containing details about the allocated memory, such as the starting address and size. If it's not possible to allocate memory meeting the criteria, returns [None].
/// # Safety
/// This function is unsafe because it directly interacts with the global allocator, which can potentially cause memory leaks or other issues if not used correctly.
pub unsafe fn alloc_coherent(layout: Layout) -> AllocResult<DMAInfo> {
    let mut mm = ALLOCATOR.lock();
    mm.alloc_coherent(layout)
}

/// Frees coherent memory previously allocated.
///
/// This function releases the memory block that was previously allocated and marked as coherent. It ensures proper deallocation and management of resources associated with the memory block.
///
/// - `dma_info`: An instance of [DMAInfo] containing the details of the memory block to be freed, such as its starting address and size.
/// # Safety
/// This function is unsafe because it directly interacts with the global allocator, which can potentially cause memory leaks or other issues if not used correctly.
pub unsafe fn dealloc_coherent(dma: DMAInfo, layout: Layout) {
    let mut mm = ALLOCATOR.lock();

    mm.dealloc_coherent(dma, layout)
}
