//! Page table manipulation.

use crate::{err::*, Arch, ArchCommon, MemRegionFlags, PageEntry, PhysAddr, VirtAddr};
use core::mem::size_of;
use page_table_entry::*;

const ENTRY_COUNT: usize = 512;

pub(crate) const fn p4_index(vaddr: VirtAddr) -> usize {
    (vaddr.as_usize() >> (12 + 27)) & (ENTRY_COUNT - 1)
}

pub(crate) const fn p3_index(vaddr: VirtAddr) -> usize {
    (vaddr.as_usize() >> (12 + 18)) & (ENTRY_COUNT - 1)
}

pub(crate) const fn p2_index(vaddr: VirtAddr) -> usize {
    (vaddr.as_usize() >> (12 + 9)) & (ENTRY_COUNT - 1)
}

pub(crate) const fn p1_index(vaddr: VirtAddr) -> usize {
    (vaddr.as_usize() >> 12) & (ENTRY_COUNT - 1)
}

/// A generic page table struct for 64-bit platform.
///
/// It also tracks all intermediate level tables. They will be deallocated
/// When the [`PageTable64`] itself is dropped.
pub struct PageTable64 {
    virt_phys_offset: usize,
    root_paddr: PhysAddr,
}

impl PageTable64 {
    pub unsafe fn new(paddr: PhysAddr, virt_phys_offset: usize) -> Self {
        let root_paddr = paddr;

        Self {
            root_paddr,
            virt_phys_offset,
        }
    }

    pub fn root_paddr(&self) -> PhysAddr {
        self.root_paddr
    }

    fn phys_to_virt(&self, paddr: PhysAddr) -> VirtAddr {
        VirtAddr::from(paddr.as_usize() + self.virt_phys_offset)
    }

    pub fn table_of<'a>(&self, paddr: PhysAddr) -> &'a [PageEntry] {
        let ptr = self.phys_to_virt(paddr).as_ptr() as _;
        unsafe { core::slice::from_raw_parts(ptr, ENTRY_COUNT) }
    }

    pub fn table_of_mut<'a>(&self, paddr: PhysAddr) -> &'a mut [PageEntry] {
        let ptr = self.phys_to_virt(paddr).as_mut_ptr() as _;
        unsafe { core::slice::from_raw_parts_mut(ptr, ENTRY_COUNT) }
    }
    pub fn next_table_mut<'a>(&self, entry: &PageEntry) -> PagingResult<&'a mut [PageEntry]> {
        if !entry.is_present() {
            Err(PagingError::NotMapped)
        } else if entry.is_huge() {
            Err(PagingError::MappedToHugePage)
        } else {
            Ok(self.table_of_mut(entry.paddr()))
        }
    }
    pub fn next_table<'a>(&self, entry: &PageEntry) -> PagingResult<&'a [PageEntry]> {
        if !entry.is_present() {
            Err(PagingError::NotMapped)
        } else if entry.is_huge() {
            Err(PagingError::MappedToHugePage)
        } else {
            Ok(self.table_of(entry.paddr()))
        }
    }
}

pub(crate) fn page_tabe_add_one_max_menory() -> usize {
    (Arch::PAGE_LEVEL - 1) * ENTRY_COUNT * size_of::<u64>()
}

// fn memory_max_pte_count(memory_size: usize, page_size: usize, page_level: usize) -> usize {
//     assert!(is_aligned_4k(memory_size));
//     let vaddr = VirtAddr::from(page_size);
//     let mut count = 0;

//     if page_level == 4 {
//         p4_index(vaddr);
//     }

//     count
// }
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

#[cfg(test)]
mod test {
    use crate::paging::memory_max_pte_count;

    #[test]
    fn test_pte_count() {
        // let count = memory_max_pte_count(2 * 1024 * 1024 * 1024, 4 * 1024, 4);
        // assert_eq!(count, 1);
    }
}
