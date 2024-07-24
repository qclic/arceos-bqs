//! Physical memory management.

use core::fmt;

#[doc(no_inline)]
pub use memory_addr::{PhysAddr, VirtAddr};
use os_dma::BusAddr;
use page_table_multiarch::MappingFlags;

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
    /// The start physical address of the region.
    pub paddr: PhysAddr,
    /// The size in bytes of the region.
    pub size: usize,
    /// The region flags, see [`MemRegionFlags`].
    pub flags: MemRegionFlags,
    /// The region name, used for identification.
    pub name: &'static str,
}

impl From<MemRegionFlags> for MappingFlags {
    fn from(f: MemRegionFlags) -> Self {
        let mut ret = Self::empty();
        if f.contains(MemRegionFlags::READ) {
            ret |= Self::READ;
        }
        if f.contains(MemRegionFlags::WRITE) {
            ret |= Self::WRITE;
        }
        if f.contains(MemRegionFlags::EXECUTE) {
            ret |= Self::EXECUTE;
        }
        if f.contains(MemRegionFlags::DEVICE) {
            ret |= Self::DEVICE;
        }
        if f.contains(MemRegionFlags::UNCACHED) {
            ret |= Self::UNCACHED;
        }
        ret
    }
}
/// Converts a virtual address to a physical address.
///
/// It assumes that there is a linear mapping with the offset
/// [`PHYS_VIRT_OFFSET`], that maps all the physical memory to the virtual
/// space at the address plus the offset. So we have
/// `paddr = vaddr - PHYS_VIRT_OFFSET`.
///
/// [`PHYS_VIRT_OFFSET`]: axconfig::PHYS_VIRT_OFFSET
#[inline]
pub const fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    PhysAddr::from(vaddr.as_usize() - axconfig::PHYS_VIRT_OFFSET)
}

/// Converts a physical address to a virtual address.
///
/// It assumes that there is a linear mapping with the offset
/// [`PHYS_VIRT_OFFSET`], that maps all the physical memory to the virtual
/// space at the address plus the offset. So we have
/// `vaddr = paddr + PHYS_VIRT_OFFSET`.
///
/// [`PHYS_VIRT_OFFSET`]: axconfig::PHYS_VIRT_OFFSET
#[inline]
pub const fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    VirtAddr::from(paddr.as_usize() + axconfig::PHYS_VIRT_OFFSET)
}

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

pub(crate) struct FreeRegion {
    pub paddr: PhysAddr,
    pub offset: usize,
    pub size: usize,
}
