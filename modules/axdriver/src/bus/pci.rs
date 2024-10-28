use crate::{prelude::*, AllDevices};
use alloc::sync::Arc;
use axdriver_pci::{
    BarInfo, Cam, Command, DeviceFunction, HeaderType, MemoryBarType, PciRangeAllocator, PciRoot,
};
use axhal::mem::phys_to_virt;
use pcie::{preludes::*, PciDevice};

const PCI_BAR_NUM: u8 = 6;

fn config_pci_device(
    root: &mut PciRoot,
    bdf: DeviceFunction,
    allocator: &mut Option<PciRangeAllocator>,
) -> DevResult {
    let mut bar = 0;
    while bar < PCI_BAR_NUM {
        let info = root.bar_info(bdf, bar).unwrap();
        if let BarInfo::Memory {
            address_type,
            address,
            size,
            ..
        } = info
        {
            // if the BAR address is not assigned, call the allocator and assign it.
            if size > 0 && address == 0 {
                let new_addr = allocator
                    .as_mut()
                    .expect("No memory ranges available for PCI BARs!")
                    .alloc(size as _)
                    .ok_or(DevError::NoMemory)?;
                if address_type == MemoryBarType::Width32 {
                    root.set_bar_32(bdf, bar, new_addr as _);
                } else if address_type == MemoryBarType::Width64 {
                    root.set_bar_64(bdf, bar, new_addr);
                }
            }
        }

        // read the BAR info again after assignment.
        let info = root.bar_info(bdf, bar).unwrap();
        match info {
            BarInfo::IO { address, size } => {
                if address > 0 && size > 0 {
                    debug!("  BAR {}: IO  [{:#x}, {:#x})", bar, address, address + size);
                }
            }
            BarInfo::Memory {
                address_type,
                prefetchable,
                address,
                size,
            } => {
                if address > 0 && size > 0 {
                    debug!(
                        "  BAR {}: MEM [{:#x}, {:#x}){}{}",
                        bar,
                        address,
                        address + size as u64,
                        if address_type == MemoryBarType::Width64 {
                            " 64bit"
                        } else {
                            ""
                        },
                        if prefetchable { " pref" } else { "" },
                    );
                }
            }
        }

        bar += 1;
        if info.takes_two_entries() {
            bar += 1;
        }
    }

    // Enable the device.
    let (_status, cmd) = root.get_status_command(bdf);
    root.set_command(
        bdf,
        cmd | Command::IO_SPACE | Command::MEMORY_SPACE | Command::BUS_MASTER,
    );
    Ok(())
}

impl AllDevices {
    pub(crate) fn probe_bus_devices(&mut self) {
        let base_vaddr = phys_to_virt(axconfig::PCI_ECAM_BASE.into());

        info!("Init PCIE @{:#X}", axconfig::PCI_ECAM_BASE);

        let mut root = pcie::RootGeneric::new(base_vaddr.as_usize());

        root.enumerate().for_each(|device| {
            let address = device.address();
            debug!("PCI {}", device);

            if let PciDevice::Endpoint(mut ep) = device {
                ep.update_command(|cmd| {
                    cmd | CommandRegister::IO_ENABLE
                        | CommandRegister::MEMORY_ENABLE
                        | CommandRegister::BUS_MASTER_ENABLE
                });
                
                let device = Arc::new(ep);

                for_each_drivers!(type Driver, {

                    if let Some(dev) = Driver::probe_pcie(&mut root, device.clone()) {
                        info!(
                            "registered a new {:?} device at {}: {:?}",
                            dev.device_type(),
                            address,
                            dev.device_name(),
                        );
                        self.add_device(dev);
                    }
                });
            }
        });
    }
}
