/// A bus memory address.
///
/// It's a wrapper type around an `usize`.
#[repr(transparent)]
#[derive(Copy, Clone, Default, Ord, PartialOrd, Eq, PartialEq)]
pub struct BusAddr(u64);

impl BusAddr {
    /// Converts an `usize` to a physical address.
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Converts the address to an `usize`.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl From<u64> for BusAddr {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}


