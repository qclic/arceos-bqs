//! Defines types and probe methods of all supported devices.

#![allow(unused_imports, dead_code)]

use crate::AxDeviceEnum;
use axdriver_base::DeviceType;

#[cfg(feature = "virtio")]
use crate::virtio::{self, VirtIoDevMeta};

#[cfg(feature = "bus-pci")]
use axdriver_pci::{DeviceFunction, DeviceFunctionInfo, PciRoot};

#[cfg(feature = "bus-pci")]
use pcie::{Chip, PciDevice, RootComplex};

pub use super::dummy::*;
#[cfg(feature = "bus-pci")]
use alloc::sync::Arc;

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

    #[cfg(bus = "pci")]
    fn probe_pcie<C: Chip>(
        _root: &mut RootComplex<C>,
        _dev: Arc<pcie::Endpoint<C>>,
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
        register_block_driver!(RamDiskDriver, axdriver_block::ramdisk::RamDisk);

        impl DriverProbe for RamDiskDriver {
            fn probe_global() -> Option<AxDeviceEnum> {
                // TODO: format RAM disk
                Some(AxDeviceEnum::from_block(
                    axdriver_block::ramdisk::RamDisk::new(0x100_0000), // 16 MiB
                ))
            }
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(block_dev = "bcm2835-sdhci")]{
        pub struct BcmSdhciDriver;
        register_block_driver!(MmckDriver, axdriver_block::bcm2835sdhci::SDHCIDriver);

        impl DriverProbe for BcmSdhciDriver {
            fn probe_global() -> Option<AxDeviceEnum> {
                debug!("mmc probe");
                axdriver_block::bcm2835sdhci::SDHCIDriver::try_new().ok().map(AxDeviceEnum::from_block)
            }
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(net_dev = "ixgbe")] {
        use crate::ixgbe::IxgbeHalImpl;
        use axhal::mem::phys_to_virt;
        pub struct IxgbeDriver;
        use pcie::preludes::*;
        register_net_driver!(IxgbeDriver, axdriver_net::ixgbe::IxgbeNic<IxgbeHalImpl, 1024, 1>);
        impl DriverProbe for IxgbeDriver {
            #[cfg(bus = "pci")]
            fn probe_pci(
                    root: &mut axdriver_pci::PciRoot,
                    bdf: axdriver_pci::DeviceFunction,
                    dev_info: &axdriver_pci::DeviceFunctionInfo,
                ) -> Option<crate::AxDeviceEnum> {
                    use axdriver_net::ixgbe::{INTEL_82599, INTEL_VEND, IxgbeNic};
                    if dev_info.vendor_id == INTEL_VEND && dev_info.device_id == INTEL_82599 {
                        // Intel 10Gb Network
                        info!("ixgbe PCI device found at {:?}", bdf);

                        // Initialize the device
                        // These can be changed according to the requirments specified in the ixgbe init function.
                        const QN: u16 = 1;
                        const QS: usize = 1024;
                        let bar_info = root.bar_info(bdf, 0).unwrap();
                        match bar_info {
                            axdriver_pci::BarInfo::Memory {
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
                            axdriver_pci::BarInfo::IO { .. } => {
                                error!("ixgbe: BAR0 is of I/O type");
                                return None;
                            }
                        }
                    }
                    None
            }
            #[cfg(bus = "pci")]
            fn probe_pcie<C: Chip>(_root: &mut RootComplex<C>, dev: & PciDevice<C>) -> Option<AxDeviceEnum> {
                use axdriver_net::ixgbe::{INTEL_82599, INTEL_VEND, IxgbeNic};
                        let (vid, did) = dev.id();
                if vid == INTEL_VEND && did == INTEL_82599 {
                        // Intel 10Gb Network
                        info!("ixgbe PCI device found at {:?}", dev.address());

                        // Initialize the device
                        // These can be changed according to the requirments specified in the ixgbe init function.
                        const QN: u16 = 1;
                        const QS: usize = 1024;
                        if let PciDeviceKind::Endpoint(ep) = &dev.kind(){
                            if let Some(bar) = ep.bar(0){
                                match bar{
                                    Bar::Memory32 { address, size, prefetchable: _ } => {
                                let ixgbe_nic = IxgbeNic::<IxgbeHalImpl, QS, QN>::init(
                                    phys_to_virt((address as usize).into()).into(),
                                    size as usize
                                )
                                .expect("failed to initialize ixgbe device");
                                return Some(AxDeviceEnum::from_net(ixgbe_nic));
                            },
                                    Bar::Memory64 { address, size, prefetchable: _ } => {
                                let ixgbe_nic = IxgbeNic::<IxgbeHalImpl, QS, QN>::init(
                                    phys_to_virt((address as usize).into()).into(),
                                    size as usize
                                )
                                .expect("failed to initialize ixgbe device");
                                return Some(AxDeviceEnum::from_net(ixgbe_nic));
                            },
                                    Bar::Io { .. } =>  {
                                        error!("ixgbe: BAR0 is of I/O type");
                                        return None;
                                    }
                                }
                            }
                        }
                    }

                None
            }
        }
    }
}

cfg_if::cfg_if! {
if #[cfg(net_dev = "e1000")] {
use axhal::mem::phys_to_virt;
use crate::e1000::E1000E;
        use pcie::preludes::*;
pub struct E1000Driver;
register_net_driver!(E1000Driver, E1000E);
impl DriverProbe for E1000Driver {
    #[cfg(bus = "pci")]
    fn probe_pci(
        root: &mut axdriver_pci::PciRoot,
        bdf: axdriver_pci::DeviceFunction,
        dev_info: &axdriver_pci::DeviceFunctionInfo,
    ) -> Option<crate::AxDeviceEnum> {
        info!("check e1000");
        None
    }
    #[cfg(bus = "pci")]
    fn probe_pcie<C: Chip>(
        _root: &mut RootComplex<C>,
        dev: Arc<pcie::Endpoint<C>>,
    ) -> Option<AxDeviceEnum> {
        let (vid, did) = dev.id();
        if vid == 0x8086 && did == 0x10D3 {
            info!("E1000E PCI device found at {:?}", dev.address());
            // Initialize the device
            // These can be changed according to the requirments specified in the ixgbe init function.
            let e1000 = E1000E::new(dev);
            return Some(AxDeviceEnum::from_net(e1000));
        }

        None
    }
}
}
}
