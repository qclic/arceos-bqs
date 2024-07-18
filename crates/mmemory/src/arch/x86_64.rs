use memory_addr::PAGE_SIZE_4K;
use page_table_entry::aarch64::A64PTE;

use super::ArchCommon;
use crate::{BootState, PageTableInfo, PhysAddr, VirtAddr};

pub struct Arch {}

impl ArchCommon for Arch {
    fn init(&self, boot_state: impl BootState) {
        todo!();
    }
    fn write_page_table_kernel(addr: PhysAddr) {
        todo!();
    }
    fn page_table_info() -> PageTableInfo {
        todo!();
    }

    const PAGE_LEVEL: usize = 4;

    const PAGE_SIZE: usize = PAGE_SIZE_4K;

    const PAGE_PA_MAX_BITS: usize = 52;
    const PAGE_VA_MAX_BITS: usize = 48;
}
impl Arch {
    pub const fn new() -> Self {
        Self {}
    }
}
