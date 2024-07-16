#![no_std]
extern crate alloc;

use allocator::global_init;
use core::fmt;
use memory_addr::PAGE_SIZE_4K;
pub use memory_addr::{PhysAddr, VirtAddr};
pub(crate) mod allocator;
pub(crate) mod arch;
pub mod err;

use arch::*;
use page_table::MappingFlags;

pub use allocator::global_allocator;

static MEMORY: MMemory = MMemory::new();

struct MMemory {
    arch: Arch,
}

impl MMemory {
    const fn new() -> Self {
        Self { arch: Arch::new() }
    }

    pub fn init_allocator(&self, boot: impl BootState) {
        let kernel_init_size = PAGE_SIZE_4K;

        // let regions = BootState::memory_regions();
        // for region in regions {
        //     if region.flags.contains(MemRegionFlags::FREE) {
        //         break;
        //     }
        // }

        // self.arch.init(boot);
    }
}

bitflags::bitflags! {
    /// The flags of a physical memory region.
    pub struct MemRegionFlags: usize {
        /// Readable.
        const READ          = 1 << 0;
        /// Writable.
        const WRITE         = 1 << 1;
        /// Executable.
        const EXECUTE       = 1 << 2;
        /// Device memory. (e.g., MMIO regions)
        const DEVICE        = 1 << 4;
        /// Uncachable memory. (e.g., framebuffer)
        const UNCACHED      = 1 << 5;
        /// Reserved memory, do not use for allocation.
        const RESERVED      = 1 << 6;
        /// Free memory for allocation.
        const FREE          = 1 << 7;
    }
}
impl fmt::Debug for MemRegionFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
/// A physical memory region.
#[derive(Debug)]
pub struct MemRegion {
    pub paddr: PhysAddr,
    pub size: usize,
    pub flags: MemRegionFlags,
    pub name: &'static str,
}

pub trait BootState {
    fn virt_to_phys(virt: VirtAddr) -> PhysAddr;
    fn memory_regions() -> impl Iterator<Item = MemRegion>;
}

pub fn init_allocator(boot_state: impl BootState) {
    MEMORY.init_allocator(boot_state);
}
