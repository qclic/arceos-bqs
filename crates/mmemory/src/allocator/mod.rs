//! [ArceOS](https://github.com/arceos-org/arceos) global memory allocator.
//!
//! It provides [`GlobalAllocator`], which implements the trait
//! [`core::alloc::GlobalAlloc`]. A static global variable of type
//! [`GlobalAllocator`] is defined with the `#[global_allocator]` attribute, to
//! be registered as the standard library’s default allocator.
pub use allocator::{BaseAllocator, ByteAllocator};

cfg_if::cfg_if! {
    if #[cfg(feature = "slab")] {
        pub type DefaultByteAllocator = allocator::SlabByteAllocator;
    } else if #[cfg(feature = "buddy")] {
        pub type DefaultByteAllocator = allocator::BuddyByteAllocator;
    } else if #[cfg(feature = "tlsf")] {
        pub type DefaultByteAllocator = allocator::TlsfByteAllocator;
    }
}

/// Returns the name of the allocator.
pub const fn name() -> &'static str {
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
