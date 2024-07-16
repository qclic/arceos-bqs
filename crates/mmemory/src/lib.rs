#![no_std]
extern crate alloc;

use core::{
    alloc::{GlobalAlloc, Layout},
    fmt,
    ptr::NonNull,
    sync::atomic::{AtomicU64, Ordering},
};
use log::debug;
use memory_addr::PAGE_SIZE_4K;
pub use memory_addr::{PhysAddr, VirtAddr};
use spinlock::SpinNoIrq;
pub(crate) mod allocator;
pub(crate) mod arch;
pub mod err;
pub(crate) mod paging;
use allocator::{MAllocator, MIN_HEAP_SIZE};
use arch::*;
use err::*;
use page_table::MappingFlags;

static MEMORY: MemoryManager = MemoryManager::new();

struct MemoryManager {
    arch: Arch,
    virt_to_phys_offset: AtomicU64,
    allocator: SpinNoIrq<MAllocator>,
}

impl MemoryManager {
    const fn new() -> Self {
        Self {
            arch: Arch::new(),
            virt_to_phys_offset: AtomicU64::new(0),
            allocator: SpinNoIrq::new(MAllocator::new()),
        }
    }

    pub fn init<B: BootState>(&self) {
        if self
            .virt_to_phys_offset
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
                if x == 0 {
                    return Some(B::virt_phys_offset() as u64);
                } else {
                    return None;
                }
            })
            .is_err()
        {
            return;
        }
        debug!("Init allocator...");

        let kernel_init_size = MIN_HEAP_SIZE;

        let regions = B::memory_regions();

        for (index, region) in regions.enumerate() {
            if region.flags.contains(MemRegionFlags::FREE) && region.size >= kernel_init_size {
                self.allocator
                    .lock()
                    .init(self.phys_to_virt(region.paddr).as_usize(), kernel_init_size);
                break;
            }
        }

        // self.arch.init(boot);
    }

    pub(crate) fn phys_to_virt(&self, addr: PhysAddr) -> VirtAddr {
        return VirtAddr::from(
            addr.as_usize() + self.virt_to_phys_offset.load(Ordering::SeqCst) as usize,
        );
    }
    pub(crate) fn virt_to_phys(&self, addr: VirtAddr) -> PhysAddr {
        return PhysAddr::from(
            addr.as_usize() - self.virt_to_phys_offset.load(Ordering::SeqCst) as usize,
        );
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
    MEMORY.allocator.lock().name()
}

impl GlobalAllocator {
    pub fn alloc(&self, layout: Layout) -> Result<NonNull<u8>> {
        MEMORY.allocator.lock().alloc(layout)
    }

    pub fn dealloc(&self, pos: NonNull<u8>, layout: Layout) {
        MEMORY.allocator.lock().dealloc(pos, layout)
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
