#![allow(unused)]

use core::{alloc::Layout, marker::PhantomData, ptr};

use crate::{alloc_coherent, dealloc_coherent, DMAInfo};

pub struct DMACoherent<T> {
    info: DMAInfo,
    _marker: PhantomData<T>,
}

impl<T> DMACoherent<T> {
    pub fn new(value: T) -> Option<Self> {
        unsafe {
            let info = alloc_coherent(Layout::for_value(&value))?;
            core::ptr::write(info.cpu_addr as *mut T, value);

            Some(Self {
                info,
                _marker: PhantomData,
            })
        }
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *const T {
        self.info.cpu_addr as *const T
    }
    #[inline(always)]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.info.cpu_addr as *mut T
    }

    #[inline(always)]
    pub fn read_volatile(&self) -> T {
        unsafe { core::ptr::read_volatile(self.as_ptr()) }
    }

    #[inline(always)]
    pub fn write_volatile(&mut self, val: T) {
        unsafe {
            core::ptr::write_volatile(self.as_mut_ptr(), val);
        }
    }
}

impl<T> Drop for DMACoherent<T> {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.as_mut_ptr());
            dealloc_coherent(self.info, Layout::new::<T>());
        }
    }
}

#[cfg(test)]
mod test {
    use core::{
        alloc::Layout,
        sync::atomic::{AtomicBool, Ordering},
    };

    use alloc::sync::Arc;

    use crate::{set_impl, DMAInfo, Impl};

    use super::DMACoherent;

    struct A {}
    impl Impl for A {
        fn alloc_coherent(layout: Layout) -> Option<crate::DMAInfo> {
            let ptr = unsafe { alloc::alloc::alloc(layout) };
            if ptr.is_null() {
                return None;
            }

            Some(DMAInfo {
                cpu_addr: ptr as usize,
                bus_addr: ptr as usize as u64,
            })
        }

        fn dealloc_coherent(dma: DMAInfo, layout: Layout) {
            unsafe { alloc::alloc::dealloc(dma.cpu_addr as *mut u8, layout) }
        }
    }

    set_impl!(A);

    struct Test {
        a: Arc<AtomicBool>,
    }

    impl Drop for Test {
        fn drop(&mut self) {
            let a = self
                .a
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |f| Some(true))
                .unwrap();

            if a == true {
                panic!("drop twice");
            }
        }
    }

    #[test]
    fn test_dma_buffer() {
        let a = Arc::new(AtomicBool::new(false));

        unsafe {
            let dma = DMACoherent::new(Test { a: a.clone() });
            drop(dma);
        }
    }
}
