#![no_std]

use core::alloc::Layout;

extern crate alloc;
mod buffer;

#[derive(Debug, Clone, Copy)]
pub struct DMAInfo {
    pub cpu_addr: usize,
    pub bus_addr: u64,
}

pub trait Impl {
    fn alloc_coherent(layout: Layout) -> Option<DMAInfo>;
    fn dealloc_coherent(dma: DMAInfo, layout: Layout);
}

pub trait Coherent {}

#[macro_export]
macro_rules! set_impl {
    ($t: ty) => {
        #[no_mangle]
        unsafe fn _os_dma_0_0_alloc_coherent(
            layout: core::alloc::Layout,
        ) -> Option<$crate::DMAInfo> {
            <$t as $crate::Impl>::alloc_coherent(layout)
        }
        #[no_mangle]
        unsafe fn _os_dma_0_0_dealloc_coherent(dma: $crate::DMAInfo, layout: core::alloc::Layout) {
            <$t as $crate::Impl>::dealloc_coherent(dma, layout);
        }
    };
}

#[inline(always)]
pub(crate) unsafe fn alloc_coherent(layout: Layout) -> Option<DMAInfo> {
    extern "Rust" {
        fn _os_dma_0_0_alloc_coherent(layout: Layout) -> Option<DMAInfo>;
    }

    #[allow(clippy::unit_arg)]
    _os_dma_0_0_alloc_coherent(layout)
}

/// Release the critical section.
///
/// This function is extremely low level. Strongly prefer using [`with`] instead.
///
/// # Safety
///
/// See [`acquire`] for the safety contract description.
#[inline(always)]
pub(crate) unsafe fn dealloc_coherent(dma: DMAInfo, layout: Layout) {
    extern "Rust" {
        fn _os_dma_0_0_dealloc_coherent(dma: DMAInfo, layout: Layout);
    }

    #[allow(clippy::unit_arg)]
    _os_dma_0_0_dealloc_coherent(dma, layout)
}
