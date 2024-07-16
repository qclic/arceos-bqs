cfg_alloc! {
    use core::alloc::Layout;
    use core::ptr::NonNull;

    pub fn ax_alloc(layout: Layout) -> Option<NonNull<u8>> {
        axruntime::alloc(layout).ok()
    }

    pub fn ax_dealloc(ptr: NonNull<u8>, layout: Layout) {
        axruntime::dealloc(ptr, layout)
    }
}
