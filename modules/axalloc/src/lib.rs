//! [ArceOS](https://github.com/arceos-org/arceos) global memory allocator.
//!
//! It provides [`GlobalAllocator`], which implements the trait
//! [`core::alloc::GlobalAlloc`]. A static global variable of type
//! [`GlobalAllocator`] is defined with the `#[global_allocator]` attribute, to
//! be registered as the standard library’s default allocator.

#![no_std]

#[macro_use]
extern crate log;
extern crate alloc;

mod mem;
mod page;
mod paging;

use alloc::vec::Vec;
use allocator::{AllocError, AllocResult, BaseAllocator, ByteAllocator};
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::NonNull;
use kspin::SpinNoIrq;
use mem::{phys_to_bus, phys_to_virt, virt_to_phys, FreeRegion};
pub use mem::{MemRegion, MemRegionFlags};
use memory_addr::{align_up, PhysAddr, VirtAddr};
pub use os_dma::DMAInfo;
pub use page_table_multiarch::{MappingFlags, PagingResult};
use paging::PageTable;

const SIZE_4K: usize = 0x1000;
const SIZE_2M: usize = 0x20_0000;
const PAGE_SIZE: usize = SIZE_4K;

pub use os_dma::BusAddr;
pub use page::GlobalPage;

cfg_if::cfg_if! {
    if #[cfg(feature = "slab")] {
        use allocator::SlabByteAllocator as DefaultByteAllocator;
    } else if #[cfg(feature = "buddy")] {
        use allocator::BuddyByteAllocator as DefaultByteAllocator;
    } else if #[cfg(feature = "tlsf")] {
        use allocator::TlsfByteAllocator as DefaultByteAllocator;
    }
}

/// The global allocator used by ArceOS.
///
/// Currently, [allocator::TlsfByteAllocator] is used as the byte allocator.
pub struct GlobalAllocator {
    mm: UnsafeCell<SpinNoIrq<Option<MemManager>>>,
}
unsafe impl Sync for GlobalAllocator {}

impl GlobalAllocator {
    /// Creates an empty [`GlobalAllocator`].
    pub const fn new() -> Self {
        Self {
            mm: UnsafeCell::new(SpinNoIrq::new(None)),
        }
    }

    /// Returns the name of the allocator.
    pub const fn name(&self) -> &'static str {
        cfg_if::cfg_if! {
            if #[cfg(feature = "slab")] {
                "slab"
            } else if #[cfg(feature = "buddy")] {
                "buddy"
            } else if #[cfg(feature = "tlsf")] {
                "TLSF"
            }
        }
    }

    fn mm_ref(&self) -> &SpinNoIrq<Option<MemManager>> {
        unsafe { &*self.mm.get() }
    }
    fn mm_mut(&self) -> &mut SpinNoIrq<Option<MemManager>> {
        unsafe { &mut *self.mm.get() }
    }

    /// Initializes the allocator with the given region.
    ///
    /// It firstly adds the whole region to the page allocator, then allocates
    /// a small region (32 KB) to initialize the byte allocator. Therefore,
    /// the given region must be larger than 32 KB.
    pub fn init<F, I, FS>(&self, regions: F, set_table_addr: FS)
    where
        F: Fn() -> I,
        I: Iterator<Item = MemRegion>,
        FS: Fn(PhysAddr),
    {
        let mut mm = self.mm_ref().lock();
        if mm.is_none() {
            debug!("Init allocator...");
            let mm_new = MemManager::new();
            *mm = Some(mm_new);
            mm.as_mut().unwrap().init(regions, set_table_addr);
        }
    }

    /// Allocate arbitrary number of bytes. Returns the left bound of the
    /// allocated region.
    ///
    /// It firstly tries to allocate from the byte allocator. If there is no
    /// memory, it asks the page allocator for more memory and adds it to the
    /// byte allocator.
    ///
    /// `align_pow2` must be a power of 2, and the returned region bound will be
    ///  aligned to it.
    pub fn alloc(&self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let mut g = self.mm_ref().lock();
        g.as_mut().unwrap().alloc(layout)
    }

    /// Gives back the allocated region to the byte allocator.
    ///
    /// The region should be allocated by [`alloc`], and `align_pow2` should be
    /// the same as the one used in [`alloc`]. Otherwise, the behavior is
    /// undefined.
    ///
    /// [`alloc`]: GlobalAllocator::alloc
    pub fn dealloc(&self, pos: NonNull<u8>, layout: Layout) {
        let mut g = self.mm_ref().lock();
        g.as_mut().unwrap().balloc.dealloc(pos, layout)
    }

    /// Allocates contiguous pages.
    ///
    /// It allocates `num_pages` pages from the page allocator.
    ///
    /// `align_pow2` must be a power of 2, and the returned region bound will be
    /// aligned to it.
    pub fn alloc_pages(&self, num_pages: usize, _align_pow2: usize) -> AllocResult<usize> {
        let layout = Layout::from_size_align(num_pages * PAGE_SIZE, PAGE_SIZE).unwrap();
        let mut g = self.mm_ref().lock();
        g.as_mut()
            .unwrap()
            .alloc(layout)
            .map(|ptr| ptr.as_ptr() as usize)
    }

    /// Gives back the allocated pages starts from `pos` to the page allocator.
    ///
    /// The pages should be allocated by [`alloc_pages`], and `align_pow2`
    /// should be the same as the one used in [`alloc_pages`]. Otherwise, the
    /// behavior is undefined.
    ///
    /// [`alloc_pages`]: GlobalAllocator::alloc_pages
    pub fn dealloc_pages(&self, pos: usize, num_pages: usize) {
        let layout = Layout::from_size_align(num_pages * PAGE_SIZE, PAGE_SIZE).unwrap();
        let mut g = self.mm_ref().lock();
        g.as_mut()
            .unwrap()
            .dealloc(NonNull::new(pos as _).unwrap(), layout);
    }

    /// Returns the number of allocated bytes in the byte allocator.
    pub fn used_bytes(&self) -> usize {
        let mut g = self.mm_ref().lock();
        g.as_mut().unwrap().balloc.used_bytes()
    }

    /// Returns the number of available bytes in the byte allocator.
    pub fn available_bytes(&self) -> usize {
        let mut g = self.mm_ref().lock();
        g.as_mut().unwrap().balloc.available_bytes()
    }

    /// Returns the number of allocated pages in the page allocator.
    pub fn used_pages(&self) -> usize {
        let mut mm = self.mm_ref().lock();
        let mm_mut = mm.as_mut().unwrap();

        mm_mut.balloc.total_bytes() / PAGE_SIZE
    }

    /// Returns the number of available pages in the page allocator.
    pub fn available_pages(&self) -> usize {
        let mut mm = self.mm_ref().lock();
        mm.as_mut()
            .unwrap()
            .free_regions
            .iter()
            .map(|r| r.size - r.offset)
            .sum::<usize>()
            / PAGE_SIZE
    }

    unsafe fn get_mem_manager_mut<'a>(&'a self) -> &'a mut MemManager {
        let mm = self.mm_mut().get_mut();
        mm.as_mut().unwrap()
    }

    pub(crate) unsafe fn alloc_nolock(&self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let mm = self.get_mem_manager_mut();
        mm.alloc(layout)
    }

    pub(crate) unsafe fn dealloc_nolock(&self, pos: NonNull<u8>, layout: Layout) {
        let mm = self.get_mem_manager_mut();
        mm.dealloc(pos, layout)
    }
}

struct MemManager {
    balloc: DefaultByteAllocator,
    coherent_alloc: DefaultByteAllocator,
    table: Option<PageTable>,
    free_regions: Vec<FreeRegion>,
}
impl MemManager {
    fn new() -> Self {
        Self {
            free_regions: Vec::new(),
            table: None,
            balloc: DefaultByteAllocator::new(),
            coherent_alloc: DefaultByteAllocator::new(),
        }
    }
    fn init<F, I, FS>(&mut self, regions: F, set_table_addr: FS)
    where
        F: Fn() -> I,
        I: Iterator<Item = MemRegion>,
        FS: Fn(PhysAddr),
    {
        let mut inited_index = None;
        let mut k_paddr = PhysAddr::from(0);
        let mut free_region_count = 0;
        let mut free_all = 0;

        for (index, region) in regions().enumerate() {
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

        let kernel_init_size = if free_all > SIZE_2M {
            SIZE_2M
        } else {
            SIZE_4K * 4
        };
        let k_vaddr = phys_to_virt(k_paddr);

        debug!(
            "initialize global allocator at: [{:#x}, {:#x})",
            k_vaddr.as_usize(),
            k_vaddr.as_usize() + kernel_init_size
        );

        self.balloc
            .add_memory(k_vaddr.as_usize(), kernel_init_size)
            .unwrap();

        let mut table = PageTable::try_new().unwrap();
        let t_paddr = table.root_paddr();

        table
            .map_region(
                k_vaddr,
                k_paddr,
                kernel_init_size,
                MappingFlags::READ | MappingFlags::WRITE,
                true,
            )
            .unwrap();

        let regions_ptr = self
            .balloc
            .alloc(Layout::array::<FreeRegion>(free_region_count).unwrap())
            .unwrap();
        let mut free_regions = unsafe {
            Vec::from_raw_parts(
                regions_ptr.as_ptr() as *mut FreeRegion,
                0,
                free_region_count,
            )
        };

        for (index, region) in regions().enumerate() {
            let vaddr = phys_to_virt(region.paddr);

            if region.flags.contains(MemRegionFlags::FREE) {
                let paddr = region.paddr;
                let size = region.size;

                let offset = if index == inited_index {
                    kernel_init_size
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
                table
                    .map_region(vaddr, region.paddr, region.size, region.flags.into(), true)
                    .unwrap();
            }
        }
        self.free_regions = free_regions;
        self.table = Some(table);
        set_table_addr(t_paddr);
    }
    fn map_free_region<'a>(&'a mut self, size: usize, flags: MappingFlags) -> Option<VirtAddr> {
        let mut addr = None;
        for r in self.free_regions.iter_mut() {
            if r.size - r.offset >= size {
                addr = Some(r.paddr + r.offset);
                r.offset += size;
                break;
            }
        }

        if let Some(addr) = addr {
            let vaddr = phys_to_virt(addr);
            self.table
                .as_mut()
                .unwrap()
                .map_region(vaddr, addr, size, flags, true)
                .unwrap();
            return Some(vaddr);
        } else {
            return None;
        }
    }
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let size = layout.size();
        // simple two-level allocator: if no heap memory, allocate from the page allocator.
        loop {
            if self.balloc.available_bytes() > SIZE_4K {
                if let Ok(ptr) = self.balloc.alloc(layout) {
                    return Ok(ptr);
                }
            }
            let expand_size = align_up(size + 1, SIZE_2M);

            let vaddr = self
                .map_free_region(expand_size, MappingFlags::READ | MappingFlags::WRITE)
                .ok_or_else(|| {
                    warn!("free memory is not enough");
                    AllocError::NoMemory
                })?;

            debug!(
                "expand heap memory: [{:#x}, {:#x})",
                vaddr,
                vaddr + expand_size
            );
            self.balloc
                .add_memory(vaddr.as_usize(), expand_size)
                .inspect_err(|e| error!("add memory fail: {e:?}"))?;
        }
    }
    fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        self.balloc.dealloc(ptr, layout)
    }

    unsafe fn alloc_coherent(&mut self, layout: Layout) -> Option<os_dma::DMAInfo> {
        let size = layout.size();
        // simple two-level allocator: if no heap memory, allocate from the page allocator.
        loop {
            if let Ok(data) = self.coherent_alloc.alloc(layout) {
                let cpu_addr = data.as_ptr() as usize;
                let paddr = virt_to_phys(VirtAddr::from(cpu_addr));
                let bus_addr = phys_to_bus(paddr);

                return Some(DMAInfo {
                    cpu_addr,
                    bus_addr: bus_addr.as_u64(),
                });
            }
            let expand_size = align_up(size, SIZE_2M);

            let vaddr = self.map_free_region(
                expand_size,
                MappingFlags::READ | MappingFlags::WRITE | MappingFlags::UNCACHED,
            )?;

            debug!(
                "expand coherent memory: [{:#x}, {:#x})",
                vaddr,
                vaddr + expand_size
            );
            self.coherent_alloc
                .add_memory(vaddr.as_usize(), expand_size)
                .inspect_err(|e| error!("add memory fail: {e:?}"))
                .unwrap();
        }
    }

    unsafe fn dealloc_coherent(&mut self, dma: DMAInfo, layout: Layout) {
        self.coherent_alloc
            .dealloc(NonNull::new_unchecked(dma.cpu_addr as *mut u8), layout)
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

#[cfg_attr(all(target_os = "none", not(test)), global_allocator)]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator::new();

/// Returns the reference to the global allocator.
pub fn global_allocator() -> &'static GlobalAllocator {
    &GLOBAL_ALLOCATOR
}

/// Initializes the global memory allocator.
///
/// This function provides initial configuration for the global memory allocator,
/// enabling it to manage memory correctly during program execution. It accepts
/// two closure parameters, one for configuring memory regions and another for
/// setting the page table address.
///
/// - `regions`: A closure that yields an iterator over memory regions. Each region
///   represents a segment of memory with a specific type.
/// - `set_table_addr`: A closure that sets the physical address of the page table
///   and flushes the translation lookaside buffer (TLB).
pub fn global_init<F, I, FS>(regions: F, set_table_addr: FS)
where
    F: Fn() -> I,
    I: Iterator<Item = MemRegion>,
    FS: Fn(PhysAddr),
{
    GLOBAL_ALLOCATOR.init(regions, set_table_addr);
}

/// Allocates `coherent` memory that meets Direct Memory Access (DMA) requirements.
///
/// This function allocates a block of memory through the global allocator. The memory pages must be contiguous, undivided, and have consistent read and write access.
///
/// - `layout`: The memory layout, which describes the size and alignment requirements of the requested memory.
///
/// Returns an [DMAInfo] structure containing details about the allocated memory, such as the starting address and size. If it's not possible to allocate memory meeting the criteria, returns [None].
pub unsafe fn alloc_coherent(layout: Layout) -> Option<DMAInfo> {
    let mut mm = GLOBAL_ALLOCATOR.mm_ref().lock();
    mm.as_mut().unwrap().alloc_coherent(layout)
}

/// Frees coherent memory previously allocated.
///
/// This function releases the memory block that was previously allocated and marked as coherent. It ensures proper deallocation and management of resources associated with the memory block.
///
/// - `dma_info`: An instance of [DMAInfo] containing the details of the memory block to be freed, such as its starting address and size.
pub unsafe fn dealloc_coherent(dma: DMAInfo, layout: Layout) {
    let mut mm = GLOBAL_ALLOCATOR.mm_ref().lock();
    mm.as_mut().unwrap().dealloc_coherent(dma, layout)
}

struct DMAAllocator;
unsafe impl os_dma::Impl for DMAAllocator {
    unsafe fn alloc_coherent(layout: Layout) -> Option<DMAInfo> {
        alloc_coherent(layout)
    }

    unsafe fn dealloc_coherent(dma: DMAInfo, layout: Layout) {
        dealloc_coherent(dma, layout)
    }
}

os_dma::set_impl!(DMAAllocator);
