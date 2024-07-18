use core::fmt;

use allocator::AllocError as AError;

#[derive(Debug)]
pub enum AllocError {
    /// Invalid `size` or `align_pow2`. (e.g. unaligned)
    InvalidParam,
    /// Memory added by `add_memory` overlapped with existed memory.
    MemoryOverlap,
    /// No enough memory to allocate.
    NoMemory,
    /// Deallocate an unallocated memory region.
    NotAllocated,
}

impl From<AError> for AllocError {
    fn from(value: AError) -> Self {
        match value {
            AError::InvalidParam => Self::InvalidParam,
            AError::MemoryOverlap => Self::MemoryOverlap,
            AError::NoMemory => Self::NoMemory,
            AError::NotAllocated => Self::NotAllocated,
        }
    }
}
/// The error type for page table operation failures.
#[derive(Debug)]
pub enum PagingError {
    /// Cannot allocate memory.
    NoMemory,
    /// The address is not aligned to the page size.
    NotAligned,
    /// The mapping is not present.
    NotMapped,
    /// The mapping is already present.
    AlreadyMapped,
    /// The page table entry represents a huge page, but the target physical
    /// frame is 4K in size.
    MappedToHugePage,
}

/// The specialized `Result` type for page table operations.
pub type PagingResult<T = ()> = core::result::Result<T, PagingError>;

pub type Result<T = ()> = core::result::Result<T, AllocError>;
