//! Defines types and probe methods of all supported devices.

#![allow(unused_imports)]

use crate::AxDeviceEnum;
use driver_common::DeviceType;
use driver_pci::Command;
use driver_xhci::{
    register_operations_init_xhci, XhciController, XhciDriverOps, VL805_DEVICE_ID, VL805_VENDOR_ID,
};

#[cfg(feature = "virtio")]
use crate::virtio::{self, VirtIoDevMeta};

#[cfg(feature = "bus-pci")]
use driver_pci::{DeviceFunction, DeviceFunctionInfo, PciRoot};

pub use super::dummy::*;

pub trait DriverProbe {
    fn probe_global() -> Option<AxDeviceEnum> {
        None
    }

    #[cfg(bus = "mmio")]
    fn probe_mmio(_mmio_base: usize, _mmio_size: usize) -> Option<AxDeviceEnum> {
        None
    }

    #[cfg(bus = "pci")]
    fn probe_pci(
        _root: &mut PciRoot,
        _bdf: DeviceFunction,
        _dev_info: &DeviceFunctionInfo,
    ) -> Option<AxDeviceEnum> {
        None
    }
}

#[cfg(net_dev = "virtio-net")]
register_net_driver!(
    <virtio::VirtIoNet as VirtIoDevMeta>::Driver,
    <virtio::VirtIoNet as VirtIoDevMeta>::Device
);

#[cfg(block_dev = "virtio-blk")]
register_block_driver!(
    <virtio::VirtIoBlk as VirtIoDevMeta>::Driver,
    <virtio::VirtIoBlk as VirtIoDevMeta>::Device
);

#[cfg(display_dev = "virtio-gpu")]
register_display_driver!(
    <virtio::VirtIoGpu as VirtIoDevMeta>::Driver,
    <virtio::VirtIoGpu as VirtIoDevMeta>::Device
);

cfg_if::cfg_if! {
    if #[cfg(block_dev = "ramdisk")] {
        pub struct RamDiskDriver;
        register_block_driver!(RamDiskDriver, driver_block::ramdisk::RamDisk);

        impl DriverProbe for RamDiskDriver {
            fn probe_global() -> Option<AxDeviceEnum> {
                // TODO: format RAM disk

                Some(AxDeviceEnum::from_block(
                    driver_block::ramdisk::RamDisk::new(0x100_0000), // 16 MiB
                ))
            }
        }
    }
}

//xhci__
cfg_if::cfg_if! {
    if #[cfg(xhci_dev = "xhci")] {
        pub struct XhciDriver;
        register_xhci_driver!(XhciDriver,driver_xhci::XhciController);

        impl DriverProbe for XhciDriver {
            fn probe_pci(
                    root: &mut PciRoot,
                    bdf: DeviceFunction,
                    dev_info: &DeviceFunctionInfo,
                ) -> Option<AxDeviceEnum> {
                    use driver_xhci::{VL805_VENDOR_ID,VL805_DEVICE_ID};
                    //todo add ah118 device detect
                    match Some((dev_info.vendor_id,dev_info.device_id)) {
                        Some((VL805_VENDOR_ID,VL805_DEVICE_ID))=>{
                            info!("vl805 found! at {:?}",bdf);
                            let bar_info = root.bar_info(bdf, 0).unwrap();
                            match bar_info {
                                driver_pci::BarInfo::Memory{address,size, ..}=>{
                                    return Some(AxDeviceEnum::XHCI(XhciController::init(address as usize)));
                                }
                                _=>return None
                            // return Some(AxDeviceEnum::from_xhci(dev))
                        }
                    }
                    _ => None
                }
            }
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(block_dev = "bcm2835-sdhci")]{
        pub struct BcmSdhciDriver;
        register_block_driver!(MmckDriver, driver_block::bcm2835sdhci::SDHCIDriver);

        impl DriverProbe for BcmSdhciDriver {
            fn probe_global() -> Option<AxDeviceEnum> {
                debug!("mmc probe");
                driver_block::bcm2835sdhci::SDHCIDriver::try_new().ok().map(AxDeviceEnum::from_block)
            }
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(net_dev = "ixgbe")] {
        use crate::ixgbe::IxgbeHalImpl;
        use axhal::mem::phys_to_virt;
        pub struct IxgbeDriver;
        register_net_driver!(IxgbeDriver, driver_net::ixgbe::IxgbeNic<IxgbeHalImpl, 1024, 1>);
        // register_xhci_driver!(XhciDriver,driver_xhci::XhciController);
        impl DriverProbe for IxgbeDriver {
            fn probe_pci(
                    root: &mut driver_pci::PciRoot,
                    bdf: driver_pci::DeviceFunction,
                    dev_info: &driver_pci::DeviceFunctionInfo,
                ) -> Option<crate::AxDeviceEnum> {
                    use crate::ixgbe::IxgbeHalImpl;
                    use driver_net::ixgbe::{INTEL_82599, INTEL_VEND, IxgbeNic};
                    //todo add ah118 device detect
                    match Some((dev_info.vendor_id,dev_info.device_id)) {
                        Some((INTEL_VEND,INTEL_82599))=> {
                            // Intel 10Gb Network
                            info!("ixgbe PCI device found at {:?}", bdf);

                            // Initialize the device
                            // These can be changed according to the requirments specified in the ixgbe init function.
                            const QN: u16 = 1;
                            const QS: usize = 1024;
                            let bar_info = root.bar_info(bdf, 0).unwrap();
                            match bar_info {
                                driver_pci::BarInfo::Memory {
                                    address,
                                    size,
                                    ..
                                } => {
                                    let ixgbe_nic = IxgbeNic::<IxgbeHalImpl, QS, QN>::init(
                                        phys_to_virt((address as usize).into()).into(),
                                        size as usize
                                    )
                                    .expect("failed to initialize ixgbe device");
                                return Some(AxDeviceEnum::from_net(ixgbe_nic));
                            }
                            driver_pci::BarInfo::IO { .. } => {
                                error!("ixgbe: BAR0 is of I/O type");
                                return None;
                            }
                        }
                        },
                        Some((VL805_VENDOR_ID,VL805_DEVICE_ID))=>{
                            info!("vl805 found! at {:?}",bdf);
                            let bar_info = root.bar_info(bdf, 0).unwrap();
                            const PCI_COMMAND_PARITY:u16 = 0x40;
                            info!("{}",bar_info);
                            unsafe {root.set_command(bdf, Command::MEMORY_SPACE|Command::BUS_MASTER|Command::SERR_ENABLE|Command::from_bits_unchecked(PCI_COMMAND_PARITY));}
                            match bar_info {
                            driver_pci::BarInfo::Memory{address,size, ..}=>{
                            let mmio = register_operations_init_xhci::enable_xhci(bdf.bus, bdf.function,  0xffff_0000_fd50_0000);
                            loop {

                                let stat = root.get_status_command(bdf).0.bits();
                                let command = root.get_status_command(bdf).1.bits();

                                info!("status:{:x}",stat);
                                info!("command:{:x}",command);
                                if stat != 0x10{
                                    break;
                                }
                            }
                                    return Some(
                                        AxDeviceEnum::XHCI(
                                            XhciController::init(
                                                mmio as usize
                                            )
                                        )
                                );
                                // return Some(AxDeviceEnum::XHCI(XhciController{}))
                                }
                                _=>return None
                            // return Some(AxDeviceEnum::from_xhci(dev))
                        }
                    }
                    _ => None
                }
            }
        }
    }
}
