#![no_std]
use core::fmt;
pub use memory_addr::{PhysAddr, VirtAddr};
pub(crate) mod arch;
use arch::*;
use page_table::MappingFlags;

static MEMORY: MMemory = MMemory::new();

struct MMemory {
    arch: Arch,
}

impl MMemory {
    const fn new() -> Self {
        Self { arch: Arch::new() }
    }

    pub fn init_allocator(&self, boot: impl BootState) {
        self.arch.init(boot);
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
