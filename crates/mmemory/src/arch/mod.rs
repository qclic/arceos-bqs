use crate::{BootState, PhysAddr, VirtAddr};
#[cfg(any(target_arch = "aarch64", doc))]
mod aarch64;


#[cfg(any(target_arch = "aarch64", doc))]
pub(crate) type Arch = aarch64::Arch;

pub(crate) trait ArchCommon {
    fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr;
    fn init(&self, boot_state: impl BootState);
}





