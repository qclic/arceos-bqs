#[doc(no_inline)]
pub use driver_common::{BaseDriverOps, DeviceType};


const VL805_VENDOR_ID: u16 = 0x1106;
const VL805_DEVICE_ID: u16 = 0x3483;
const VL805_MMIO_BASE: usize = 0x6_0000_0000;



pub struct  VL805{}



impl  BaseDriverOps for VL805{
    fn device_name(&self) -> &str {
        "VL805 4-Port USB 3.0 Host Controller"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::XHCI
    }
}

impl VL805 {
    pub fn probe_pci(
        vendor_id: u16, device_id: u16
    ) -> Option<Self> {
        if !(vendor_id == VL805_VENDOR_ID && device_id == VL805_DEVICE_ID) {
            return None;
        } 


            


        Some(Self {})
    }
}