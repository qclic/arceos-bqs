use memory_addr::*;
pub use memory_addr::{PhysAddr, VirtAddr};

/// A bus memory address.
///
/// It's a wrapper type around an `usize`.
#[repr(transparent)]
#[derive(Copy, Clone, Default, Ord, PartialOrd, Eq, PartialEq)]
pub struct BusAddr(usize);

impl BusAddr {
    /// Converts an `usize` to a physical address.
    #[inline]
    pub const fn from(addr: usize) -> Self {
        Self(addr)
    }

    /// Converts the address to an `usize`.
    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0
    }

    /// Aligns the address downwards to the given alignment.
    ///
    /// See the [`align_down`] function for more information.
    #[inline]
    pub fn align_down<U>(self, align: U) -> Self
    where
        U: Into<usize>,
    {
        Self(align_down(self.0, align.into()))
    }

    /// Aligns the address upwards to the given alignment.
    ///
    /// See the [`align_up`] function for more information.
    #[inline]
    pub fn align_up<U>(self, align: U) -> Self
    where
        U: Into<usize>,
    {
        Self(align_up(self.0, align.into()))
    }

    /// Returns the offset of the address within the given alignment.
    ///
    /// See the [`align_offset`] function for more information.
    #[inline]
    pub fn align_offset<U>(self, align: U) -> usize
    where
        U: Into<usize>,
    {
        align_offset(self.0, align.into())
    }

    /// Checks whether the address has the demanded alignment.
    ///
    /// See the [`is_aligned`] function for more information.
    #[inline]
    pub fn is_aligned<U>(self, align: U) -> bool
    where
        U: Into<usize>,
    {
        is_aligned(self.0, align.into())
    }

    /// Aligns the address downwards to 4096 (bytes).
    #[inline]
    pub fn align_down_4k(self) -> Self {
        self.align_down(PAGE_SIZE_4K)
    }

    /// Aligns the address upwards to 4096 (bytes).
    #[inline]
    pub fn align_up_4k(self) -> Self {
        self.align_up(PAGE_SIZE_4K)
    }

    /// Returns the offset of the address within a 4K-sized page.
    #[inline]
    pub fn align_offset_4k(self) -> usize {
        self.align_offset(PAGE_SIZE_4K)
    }

    /// Checks whether the address is 4K-aligned.
    #[inline]
    pub fn is_aligned_4k(self) -> bool {
        self.is_aligned(PAGE_SIZE_4K)
    }
}
