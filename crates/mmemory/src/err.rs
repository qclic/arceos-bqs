pub use allocator::AllocError;

pub type Result<T = ()> = core::result::Result<T, AllocError>;
