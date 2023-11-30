//! Structures and functions for PCI bus operations.
//!
//! Currently, it just re-exports structures from the crate [virtio-drivers][1]
//! and its module [`virtio_drivers::transport::pci::bus`][2].
//!
//! [1]: https://docs.rs/virtio-drivers/latest/virtio_drivers/
//! [2]: https://docs.rs/virtio-drivers/latest/virtio_drivers/transport/pci/bus/index.html

#![no_std]

use core::mem::size_of;
use core::ptr::{self, NonNull};
use core::sync::atomic::AtomicUsize;

use axhal::mem::{virt_to_phys, VirtAddr};
use fdt::node::FdtNode;
use fdt::standard_nodes::Compatible;
use fdt::Fdt;
use lazy_init::LazyInit;
use log::{debug, info, trace, warn};
use virtio_drivers::transport::mmio::{MmioTransport, VirtIOHeader};
pub use virtio_drivers::transport::pci::bus::{BarInfo, Cam, HeaderType, MemoryBarType, PciError};
pub use virtio_drivers::transport::pci::bus::{
    CapabilityInfo, Command, DeviceFunction, DeviceFunctionInfo, PciRoot, Status,
};
use virtio_drivers::transport::pci::{virtio_device_type, PciTransport};
use virtio_drivers::transport::Transport;
use virtio_drivers::{BufferDirection, Hal, PhysAddr, PAGE_SIZE};

pub fn op_spec_mmio_device<OpMMIODevice, OpPCIDevice, DeviceFilter>(
    fdt_file: usize,
    op_mmio_device: OpMMIODevice,
    op_pci_device: OpPCIDevice,
    device_filter: DeviceFilter,
) where
    OpMMIODevice: Fn(MmioTransport),
    OpPCIDevice: Fn(PciTransport),
    DeviceFilter: Fn(&DeviceFunctionInfo) -> bool,
{
    // info!("driver pci started.");
    // info!("Loading FDT from {:x}", mmio_base);
    // Safe because the pointer is a valid pointer to unaliased memory.
    // let fdt = unsafe { Fdt::from_ptr(mmio_base as *const u8).unwrap() }; // fix this stuff
    let fdt = unsafe { Fdt::from_ptr(fdt_file as *const u8).unwrap() };

    for node in fdt.all_nodes() {
        // Dump information about the node for debugging.
        trace!(
            "{}: {:?}",
            node.name,
            node.compatible().map(Compatible::first),
        );
        if let Some(reg) = node.reg() {
            for range in reg {
                trace!(
                    "  {:#018x?}, length {:?}",
                    range.starting_address,
                    range.size
                );
            }
        }

        // Check whether it is a VirtIO MMIO device.
        if let (Some(compatible), Some(region)) =
            (node.compatible(), node.reg().and_then(|mut reg| reg.next()))
        {
            if compatible.all().any(|s| s == "virtio,mmio")
                && region.size.unwrap_or(0) > size_of::<VirtIOHeader>()
            {
                debug!("Found VirtIO MMIO device at {:?}", region);

                let header = NonNull::new(region.starting_address as *mut VirtIOHeader).unwrap();
                match unsafe { MmioTransport::new(header) } {
                    Err(e) => warn!("Error creating VirtIO MMIO transport: {}", e),
                    Ok(transport) => {
                        // if device_filter(&transport) {
                        info!(
                            "Detected spec mmio device with vendor id {:#X}, device type {:?}, version {:?}",
                            transport.vendor_id(),
                            transport.device_type(),
                            transport.version(),
                        );
                        op_mmio_device(transport);
                        // }
                    }
                }
            }
        }
    }

    if let Some(pci_node) = fdt.find_compatible(&["pci-host-cam-generic"]) {
        // info!("Found PCI node: {}", pci_node.name);
        enumerate_pci(pci_node, Cam::MmioCam, &op_pci_device, &device_filter);
    }
    if let Some(pcie_node) = fdt.find_compatible(&["pci-host-ecam-generic"]) {
        // info!("Found PCIe node: {}", pcie_node.name);
        enumerate_pci(pcie_node, Cam::Ecam, &op_pci_device, &device_filter);
    }
}
pub struct HalImpl;
extern "C" {
    static dma_region: u8;
}

static DMA_PADDR: LazyInit<AtomicUsize> = LazyInit::new();

unsafe impl Hal for HalImpl {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (PhysAddr, NonNull<u8>) {
        DMA_PADDR.init_by(AtomicUsize::new(unsafe {
            &dma_region as *const u8 as usize
        }));
        let paddr = DMA_PADDR.fetch_add(PAGE_SIZE * pages, core::sync::atomic::Ordering::SeqCst);
        trace!("alloc DMA: paddr={:#x}, pages={}", paddr, pages);
        let vaddr = NonNull::new(paddr as _).unwrap();
        (paddr, vaddr)
    }

    unsafe fn dma_dealloc(paddr: PhysAddr, _vaddr: NonNull<u8>, pages: usize) -> i32 {
        trace!("dealloc DMA: paddr={:#x}, pages={}", paddr, pages);
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: PhysAddr, _size: usize) -> NonNull<u8> {
        NonNull::new(paddr as _).unwrap()
    }

    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> PhysAddr {
        let vaddr = buffer.as_ptr() as *mut u8 as usize;
        // Nothing to do, as the host already has access to all memory.
        virt_to_phys(VirtAddr::from(vaddr)).as_usize()
    }

    unsafe fn unshare(_paddr: PhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {
        // Nothing to do, as the host already has access to all memory and we didn't copy the buffer
        // anywhere else.
    }
}

fn enumerate_pci<OpDevice, DeviceFilter>(
    pci_node: FdtNode,
    cam: Cam,
    op_device: OpDevice,
    device_filter: DeviceFilter,
) where
    OpDevice: Fn(PciTransport),
    DeviceFilter: Fn(&DeviceFunctionInfo) -> bool,
{
    let reg = pci_node.reg().expect("PCI node missing reg property.");
    let mut allocator = PciMemory32Allocator::for_pci_ranges(&pci_node);

    for region in reg {
        info!(
            "Reg: {:?}-{:#x}",
            region.starting_address,
            region.starting_address as usize + region.size.unwrap()
        );
        assert_eq!(region.size.unwrap(), cam.size() as usize);
        // Safe because we know the pointer is to a valid MMIO region.
        let mut pci_root = unsafe { PciRoot::new(region.starting_address as *mut u8, cam) };
        for (device_function, info) in pci_root.enumerate_bus(0) {
            if device_filter(&info) {
                let (status, command) = pci_root.get_status_command(device_function);
                info!(
                    "Found {} at {}, status {:?} command {:?}",
                    info, device_function, status, command
                );
                if let Some(virtio_type) = virtio_device_type(&info) {
                    info!("  VirtIO {:?}", virtio_type);
                    allocate_bars(&mut pci_root, device_function, &mut allocator);
                    dump_bar_contents(&mut pci_root, device_function, 4);
                    let mut transport =
                        PciTransport::new::<HalImpl>(&mut pci_root, device_function).unwrap();
                    info!(
                        "Detected virtio PCI device with device type {:?}, features {:#018x}",
                        transport.device_type(),
                        transport.read_device_features(),
                    );
                    op_device(transport);
                }
            }
        }
    }
}

/// Allocates 32-bit memory addresses for PCI BARs.
struct PciMemory32Allocator {
    start: u32,
    end: u32,
}
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum PciRangeType {
    ConfigurationSpace,
    IoSpace,
    Memory32,
    Memory64,
}
impl From<u8> for PciRangeType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::ConfigurationSpace,
            1 => Self::IoSpace,
            2 => Self::Memory32,
            3 => Self::Memory64,
            _ => panic!("Tried to convert invalid range type {}", value),
        }
    }
}
impl PciMemory32Allocator {
    /// Creates a new allocator based on the ranges property of the given PCI node.
    pub fn for_pci_ranges(pci_node: &FdtNode) -> Self {
        let ranges = pci_node
            .property("ranges")
            .expect("PCI node missing ranges property.");
        let mut memory_32_address = 0;
        let mut memory_32_size = 0;
        for i in 0..ranges.value.len() / 28 {
            let range = &ranges.value[i * 28..(i + 1) * 28];
            let prefetchable = range[0] & 0x80 != 0;
            let range_type = PciRangeType::from(range[0] & 0x3);
            let bus_address = u64::from_be_bytes(range[4..12].try_into().unwrap());
            let cpu_physical = u64::from_be_bytes(range[12..20].try_into().unwrap());
            let size = u64::from_be_bytes(range[20..28].try_into().unwrap());
            info!(
                "range: {:?} {}prefetchable bus address: {:#018x} host physical address: {:#018x} size: {:#018x}",
                range_type,
                if prefetchable { "" } else { "non-" },
                bus_address,
                cpu_physical,
                size,
            );
            // Use the largest range within the 32-bit address space for 32-bit memory, even if it
            // is marked as a 64-bit range. This is necessary because crosvm doesn't currently
            // provide any 32-bit ranges.
            if !prefetchable
                && matches!(range_type, PciRangeType::Memory32 | PciRangeType::Memory64)
                && size > memory_32_size.into()
                && bus_address + size < u32::MAX.into()
            {
                assert_eq!(bus_address, cpu_physical);
                memory_32_address = u32::try_from(cpu_physical).unwrap();
                memory_32_size = u32::try_from(size).unwrap();
            }
        }
        if memory_32_size == 0 {
            panic!("No 32-bit PCI memory region found.");
        }
        Self {
            start: memory_32_address,
            end: memory_32_address + memory_32_size,
        }
    }

    /// Allocates a 32-bit memory address region for a PCI BAR of the given power-of-2 size.
    ///
    /// It will have alignment matching the size. The size must be a power of 2.
    pub fn allocate_memory_32(&mut self, size: u32) -> u32 {
        assert!(size.is_power_of_two());
        let allocated_address = align_up(self.start, size);
        assert!(allocated_address + size <= self.end);
        self.start = allocated_address + size;
        allocated_address
    }
}

const fn align_up(value: u32, alignment: u32) -> u32 {
    ((value - 1) | (alignment - 1)) + 1
}

fn dump_bar_contents(root: &mut PciRoot, device_function: DeviceFunction, bar_index: u8) {
    let bar_info = root.bar_info(device_function, bar_index).unwrap();
    trace!("Dumping bar {}: {:#x?}", bar_index, bar_info);
    if let BarInfo::Memory { address, size, .. } = bar_info {
        let start = address as *const u8;
        unsafe {
            let mut buf = [0u8; 32];
            for i in 0..size / 32 {
                let ptr = start.add(i as usize * 32);
                ptr::copy(ptr, buf.as_mut_ptr(), 32);
                if buf.iter().any(|b| *b != 0xff) {
                    trace!("  {:?}: {:x?}", ptr, buf);
                }
            }
        }
    }
    trace!("End of dump");
}

/// Allocates appropriately-sized memory regions and assigns them to the device's BARs.
fn allocate_bars(
    root: &mut PciRoot,
    device_function: DeviceFunction,
    allocator: &mut PciMemory32Allocator,
) {
    let mut bar_index = 0;
    while bar_index < 6 {
        let info = root.bar_info(device_function, bar_index).unwrap();
        debug!("BAR {}: {}", bar_index, info);
        // Ignore I/O bars, as they aren't required for the VirtIO driver.
        if let BarInfo::Memory {
            address_type, size, ..
        } = info
        {
            match address_type {
                MemoryBarType::Width32 => {
                    if size > 0 {
                        let address = allocator.allocate_memory_32(size);
                        debug!("Allocated address {:#010x}", address);
                        root.set_bar_32(device_function, bar_index, address);
                    }
                }
                MemoryBarType::Width64 => {
                    if size > 0 {
                        let address = allocator.allocate_memory_32(size);
                        debug!("Allocated address {:#010x}", address);
                        root.set_bar_64(device_function, bar_index, address.into());
                    }
                }

                _ => panic!("Memory BAR address type {:?} not supported.", address_type),
            }
        }

        bar_index += 1;
        if info.takes_two_entries() {
            bar_index += 1;
        }
    }

    // Enable the device to use its BARs.
    root.set_command(
        device_function,
        Command::IO_SPACE | Command::MEMORY_SPACE | Command::BUS_MASTER,
    );
    let (status, command) = root.get_status_command(device_function);
    debug!(
        "Allocated BARs and enabled device, status {:?} command {:?}",
        status, command
    );
}

/// Used to allocate MMIO regions for PCI BARs.
pub struct PciRangeAllocator {
    _start: u64,
    end: u64,
    current: u64,
}

impl PciRangeAllocator {
    /// Creates a new allocator from a memory range.
    pub const fn new(base: u64, size: u64) -> Self {
        Self {
            _start: base,
            end: base + size,
            current: base,
        }
    }

    /// Allocates a memory region with the given size.
    ///
    /// The `size` should be a power of 2, and the returned value is also a
    /// multiple of `size`.
    pub fn alloc(&mut self, size: u64) -> Option<u64> {
        if !size.is_power_of_two() {
            return None;
        }
        let ret = align_up_orig(self.current, size);
        if ret + size > self.end {
            return None;
        }

        self.current = ret + size;
        Some(ret)
    }
}

const fn align_up_orig(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}
