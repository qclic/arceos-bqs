#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use core::{
    alloc::{GlobalAlloc, Layout},
    fmt,
    ptr::NonNull,
};
use log::{debug, error, trace};
use memory_addr::{align_up, PAGE_SIZE_4K};
pub use memory_addr::{PhysAddr, VirtAddr};
use page_table::PageSize;
use page_table_entry::*;
use paging::PageTable64;
use spinlock::SpinNoIrq;
pub(crate) mod allocator;
pub(crate) mod arch;
pub mod err;
pub(crate) mod paging;
use allocator::*;
use arch::*;
use err::*;
use paging::*;

static MEMORY: MemoryManager = MemoryManager::new();

struct MemoryManager {
    inner: SpinNoIrq<Option<Inner>>,
}

struct FreeRegion {
    paddr: PhysAddr,
    offset: usize,
    size: usize,
}

struct Inner {
    allocator: DefaultByteAllocator,
    table: Option<PageTable64>,
    virt_phys_offset: usize,
    free_regions: Vec<FreeRegion>,
}

impl MemoryManager {
    const fn new() -> Self {
        Self {
            inner: SpinNoIrq::new(None),
        }
    }

    pub fn init<B: BootState>(&self) {
        let mut inner = self.inner.lock();
        if inner.is_none() {
            debug!("Init allocator...");
            let mut mm = Inner::new::<B>();
            mm.init::<B>();
            *inner = Some(mm);
        }
        debug!("Init ok");
    }
}

impl Inner {
    fn new<B: BootState>() -> Self {
        let virt_phys_offset = B::virt_phys_offset();

        let free_regions = Vec::new();
        let allocator = DefaultByteAllocator::new();

        Self {
            allocator,
            free_regions,
            virt_phys_offset,
            table: None,
        }
    }

    fn init<B: BootState>(&mut self) {
        if self.table.is_some() {
            return;
        }
        let mut inited_index = None;
        let mut k_paddr = PhysAddr::from(0);
        let mut free_region_count = 0;
        let mut free_all = 0;

        for (index, region) in B::memory_regions().enumerate() {
            if region.flags.contains(MemRegionFlags::FREE) {
                if inited_index.is_none() {
                    inited_index = Some(index);
                    k_paddr = region.paddr;
                }
                free_region_count += 1;
                free_all += region.size;
                break;
            }
        }
        let inited_index = inited_index.expect("No enough memory for kernel initialization");
        let kernel_init_size = if free_all > PageSize::Size2M as usize {
            PageSize::Size2M as usize
        } else {
            PAGE_SIZE_4K * 12
        };
        let t_paddr;
        unsafe {
            self.allocator
                .init(self.phys_to_virt(k_paddr).as_usize(), kernel_init_size);

            let t_vaddr = self.new_table_frame();
            t_paddr = self.virt_to_phys(t_vaddr);

            let table = PageTable64::new(t_paddr, self.virt_phys_offset);
            self.table = Some(table);
            self.map_region(
                t_vaddr,
                t_paddr,
                kernel_init_size,
                MappingFlags::READ | MappingFlags::WRITE,
                true,
            )
            .unwrap();
        }

        let regions_ptr = self
            .allocator
            .alloc(Layout::array::<FreeRegion>(free_region_count).unwrap())
            .unwrap();
        let mut free_regions = unsafe {
            Vec::from_raw_parts(
                regions_ptr.as_ptr() as *mut FreeRegion,
                0,
                free_region_count,
            )
        };

        for (index, region) in B::memory_regions().enumerate() {
            let vaddr = self.phys_to_virt(region.paddr);

            if region.flags.contains(MemRegionFlags::FREE) {
                let paddr = region.paddr;
                let size = region.size;

                let offset = if index == inited_index {
                    kernel_init_size + (t_paddr.as_usize() - paddr.as_usize())
                } else {
                    0
                };
                let free_region = FreeRegion {
                    paddr,
                    offset,
                    size,
                };

                free_regions.push(free_region);
            } else {
                self.map_region(vaddr, region.paddr, region.size, region.flags.into(), true)
                    .unwrap();
            }
        }
        self.free_regions = free_regions;
        let table_addr = self.table().root_paddr();
        debug!("Init memory manager ok, {table_addr:?}");

        Arch::write_page_table_kernel(table_addr);
        Arch::flush_tlb(None);
    }

    unsafe fn new_table_frame(&mut self) -> VirtAddr {
        let data = self
            .allocator
            .alloc(Layout::from_size_align(PAGE_SIZE_4K, PAGE_SIZE_4K).unwrap())
            .unwrap();
        data.as_ptr().write_bytes(0, PAGE_SIZE_4K);
        VirtAddr::from(data.as_ptr() as usize)
    }

    fn print_table(&self) {
        self.walk(100, &|level, index, vaddr, pte| {
            trace!(
                "{}index: {} {:?}",
                match level {
                    0 => "",
                    1 => "-",
                    2 => "--",
                    _ => "---",
                },
                index,
                vaddr
            );
        })
        .unwrap();
    }

    fn table<'a>(&'a self) -> &'a PageTable64 {
        self.table.as_ref().unwrap()
    }
    fn table_mut<'a>(&'a mut self) -> &'a mut PageTable64 {
        self.table.as_mut().unwrap()
    }

    #[inline(always)]
    fn table_of_mut<'a>(&mut self, paddr: PhysAddr) -> &'a mut [PageEntry] {
        self.table.as_mut().unwrap().table_of_mut(paddr)
    }
    #[inline(always)]
    fn next_table_mut<'a>(&mut self, entry: &PageEntry) -> PagingResult<&'a mut [PageEntry]> {
        self.table.as_mut().unwrap().next_table_mut(entry)
    }

    unsafe fn next_table_mut_or_create<'a>(
        &mut self,
        entry: &'a mut PageEntry,
    ) -> PagingResult<&'a mut [PageEntry]> {
        if entry.is_unused() {
            let vaddr = self.new_table_frame();
            let paddr = self.virt_to_phys(vaddr);
            *entry = GenericPTE::new_table(paddr);
            Ok(self.table_of_mut(paddr))
        } else {
            self.next_table_mut(entry)
        }
    }

    unsafe fn get_entry_mut_or_create(
        &mut self,
        vaddr: VirtAddr,
        page_size: PageSize,
    ) -> PagingResult<&mut PageEntry> {
        let p3 = if Arch::PAGE_LEVEL == 3 {
            self.table_of_mut(self.table().root_paddr())
        } else if Arch::PAGE_LEVEL == 4 {
            let p4 = self.table_of_mut(self.table().root_paddr());
            let p4e = &mut p4[p4_index(vaddr)];
            self.next_table_mut_or_create(p4e)?
        } else {
            unreachable!()
        };
        let p3e = &mut p3[p3_index(vaddr)];
        if page_size == PageSize::Size1G {
            return Ok(p3e);
        }

        let p2 = self.next_table_mut_or_create(p3e)?;
        let p2e = &mut p2[p2_index(vaddr)];
        if page_size == PageSize::Size2M {
            return Ok(p2e);
        }

        let p1 = self.next_table_mut_or_create(p2e)?;
        let p1e = &mut p1[p1_index(vaddr)];
        Ok(p1e)
    }
    /// Maps a virtual page to a physical frame with the given `page_size`
    /// and mapping `flags`.
    ///
    /// The virtual page starts with `vaddr`, amd the physical frame starts with
    /// `target`. If the addresses is not aligned to the page size, they will be
    /// aligned down automatically.
    ///
    /// Returns [`Err(PagingError::AlreadyMapped)`](PagingError::AlreadyMapped)
    /// if the mapping is already present.
    pub fn map(
        &mut self,
        vaddr: VirtAddr,
        target: PhysAddr,
        page_size: PageSize,
        flags: MappingFlags,
    ) -> PagingResult {
        // trace!("map {:x} -> {:x} {:?}", vaddr, target, page_size);
        let entry = unsafe { self.get_entry_mut_or_create(vaddr, page_size)? };
        if !entry.is_unused() {
            return Err(PagingError::AlreadyMapped);
        }

        *entry = GenericPTE::new_page(target.align_down(page_size), flags, page_size.is_huge());
        Ok(())
    }
    /// Map a contiguous virtual memory region to a contiguous physical memory
    /// region with the given mapping `flags`.
    ///
    /// The virtual and physical memory regions start with `vaddr` and `paddr`
    /// respectively. The region size is `size`. The addresses and `size` must
    /// be aligned to 4K, otherwise it will return [`Err(PagingError::NotAligned)`].
    ///
    /// When `allow_huge` is true, it will try to map the region with huge pages
    /// if possible. Otherwise, it will map the region with 4K pages.
    ///
    /// [`Err(PagingError::NotAligned)`]: PagingError::NotAligned
    pub fn map_region(
        &mut self,
        vaddr: VirtAddr,
        paddr: PhysAddr,
        size: usize,
        flags: MappingFlags,
        allow_huge: bool,
    ) -> PagingResult {
        if !vaddr.is_aligned(PageSize::Size4K)
            || !paddr.is_aligned(PageSize::Size4K)
            || !memory_addr::is_aligned(size, PageSize::Size4K.into())
        {
            return Err(PagingError::NotAligned);
        }
        trace!(
            "map_region({:#x}): [{:#x}, {:#x}) -> [{:#x}, {:#x}) {:?}",
            self.table().root_paddr(),
            vaddr,
            vaddr + size,
            paddr,
            paddr + size,
            flags,
        );
        let mut vaddr = vaddr;
        let mut paddr = paddr;
        let mut size = size;
        while size > 0 {
            let page_size = if allow_huge {
                if vaddr.is_aligned(PageSize::Size1G)
                    && paddr.is_aligned(PageSize::Size1G)
                    && size >= PageSize::Size1G as usize
                {
                    PageSize::Size1G
                } else if vaddr.is_aligned(PageSize::Size2M)
                    && paddr.is_aligned(PageSize::Size2M)
                    && size >= PageSize::Size2M as usize
                {
                    PageSize::Size2M
                } else {
                    PageSize::Size4K
                }
            } else {
                PageSize::Size4K
            };
            self.map(vaddr, paddr, page_size, flags).inspect_err(|e| {
                error!(
                    "failed to map page: {:#x?}({:?}) -> {:#x?}, {:?}",
                    vaddr, page_size, paddr, e
                )
            })?;
            vaddr += page_size as usize;
            paddr += page_size as usize;
            size -= page_size as usize;
        }
        Ok(())
    }

    pub fn phys_to_virt(&self, addr: PhysAddr) -> VirtAddr {
        return VirtAddr::from(addr.as_usize() + self.virt_phys_offset);
    }
    pub fn virt_to_phys(&self, addr: VirtAddr) -> PhysAddr {
        return PhysAddr::from(addr.as_usize() - self.virt_phys_offset);
    }

    fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>> {
        if self.allocator.available_bytes() < PAGE_SIZE_4K * 2
            || self.allocator.available_bytes() < layout.size()
        {
            let mut size = layout.size().max(PAGE_SIZE_4K * 2);
            size = align_up(size, PageSize::Size2M as usize);

            trace!("memory is not enough, try to allocate more memory {size}");

            let mut addr = None;
            for r in self.free_regions.iter_mut() {
                if r.size - r.offset >= size {
                    addr = Some(r.paddr + r.offset);
                    r.offset += size;
                    break;
                }
            }

            if let Some(addr) = addr {
                let vaddr = self.phys_to_virt(addr);
                self.map_region(
                    vaddr,
                    addr,
                    size,
                    MappingFlags::READ | MappingFlags::WRITE,
                    true,
                )
                .map_err(|e| {
                    error!("map region fail: {:?}", e);
                    AllocError::NoMemory
                })?;
                self.allocator.add_memory(vaddr.as_usize(), size)?;
            } else {
                return Err(AllocError::NoMemory);
            }
        }
        Ok(self.allocator.alloc(layout)?)
    }
    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
        self.allocator.dealloc(pos, layout)
    }
    fn walk_recursive<F>(
        &self,
        table: &[PageEntry],
        level: usize,
        start_vaddr: VirtAddr,
        limit: usize,
        func: &F,
    ) -> PagingResult
    where
        F: Fn(usize, usize, VirtAddr, &PageEntry),
    {
        let mut n = 0;
        for (i, entry) in table.iter().enumerate() {
            let vaddr = start_vaddr + (i << (12 + (Arch::PAGE_LEVEL - 1 - level) * 9));
            if entry.is_present() {
                func(level, i, vaddr, entry);
                if level < Arch::PAGE_LEVEL - 1 && !entry.is_huge() {
                    let table_entry = self.table().next_table(entry)?;
                    self.walk_recursive(table_entry, level + 1, vaddr, limit, func)?;
                }
                n += 1;
                if n >= limit {
                    break;
                }
            }
        }
        Ok(())
    }
    /// Walk the page table recursively.
    ///
    /// When reaching the leaf page table, call `func` on the current page table
    /// entry. The max number of enumerations in one table is limited by `limit`.
    ///
    /// The arguments of `func` are:
    /// - Current level (starts with `0`): `usize`
    /// - The index of the entry in the current-level table: `usize`
    /// - The virtual address that is mapped to the entry: [`VirtAddr`]
    /// - The reference of the entry: [`&PTE`](GenericPTE)
    pub fn walk<F>(&self, limit: usize, func: &F) -> PagingResult
    where
        F: Fn(usize, usize, VirtAddr, &PageEntry),
    {
        self.walk_recursive(
            self.table().table_of(self.table().root_paddr()),
            0,
            VirtAddr::from(self.virt_phys_offset),
            limit,
            func,
        )
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
    fn virt_phys_offset() -> usize;
    fn memory_regions() -> impl Iterator<Item = MemRegion>;
}

pub fn init_allocator<B: BootState>() {
    MEMORY.init::<B>();
}

pub struct GlobalAllocator;

#[cfg_attr(all(target_os = "none", not(test)), global_allocator)]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator {};

/// Returns the reference to the global allocator.
pub fn global_allocator() -> &'static GlobalAllocator {
    &GLOBAL_ALLOCATOR
}

pub fn allocator_name() -> &'static str {
    allocator::name()
}

impl GlobalAllocator {
    pub fn alloc(&self, layout: Layout) -> Result<NonNull<u8>> {
        let mut mm = MEMORY.inner.lock();
        let mm = mm.as_mut().expect("allocator not initialized");
        mm.alloc(layout)
    }

    pub fn dealloc(&self, pos: NonNull<u8>, layout: Layout) {
        let mut mm = MEMORY.inner.lock();
        mm.as_mut().unwrap().dealloc(pos, layout)
    }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if let Ok(ptr) = GlobalAllocator::alloc(self, layout) {
            ptr.as_ptr()
        } else {
            alloc::alloc::handle_alloc_error(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        GlobalAllocator::dealloc(self, NonNull::new(ptr).expect("dealloc null ptr"), layout)
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn it_works() {
        assert!(true);
    }
}
