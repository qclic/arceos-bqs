//! Common traits and types for xhci device drivers.

#![no_std]
#![feature(strict_provenance)]

use core::{
    alloc::{GlobalAlloc, Layout},
    num::NonZeroUsize,
    time::Duration,
};

use alloc::boxed::Box;
use axalloc::GlobalAllocator;
use axhal::{
    mem::{phys_to_virt, virt_to_phys, PhysAddr, VirtAddr},
    time::busy_wait,
};
#[doc(no_inline)]
pub use driver_common::{BaseDriverOps, DevError, DevResult, DeviceType};
use log::info;
use page_table_entry::{aarch64::A64PTE, GenericPTE, MappingFlags};
use xhci::{
    accessor::Mapper,
    extended_capabilities::xhci_supported_protocol,
    registers::{operational::UsbStatusRegister, Operational},
    Registers,
};

pub struct XhciController {
    pub controller: Option<Registers<MemoryMapper>>,
}

pub const VL805_VENDOR_ID: u16 = 0x1106;
pub const VL805_DEVICE_ID: u16 = 0x3483;
pub const VL805_MMIO_BASE: usize = 0x6_0000_0000;

pub mod register_operations_init_xhci;

/// The information of the graphics device.
#[derive(Debug, Clone, Copy)]
pub struct XhciInfo {}

#[derive(Clone)]
struct MemoryMapper {
    // addr_offset: usize,
}

impl Mapper for MemoryMapper {
    unsafe fn map(&mut self, phys_base: usize, bytes: usize) -> NonZeroUsize {
        // let virt_to_phys = virt_to_phys(phys_base.into());
        // let from = A64PTE(phys_base);

        // info!("mapping");
        // let pte: A64PTE =
        //     page_table::GenericPTE::new_page(virt_to_phys, MappingFlags::DEVICE, false);
        // // A64PTE::
        // info!("mapped");
        // let phys_to_virt = page_table::PagingIf::phys_to_virt(PhysAddr::from(phys_base));
        info!("mapping:{:x}", phys_base);

        // return NonZeroUsize::new_unchecked(phys_base + self.addr_offset);
        return NonZeroUsize::new_unchecked(phys_to_virt(phys_base.into()).as_usize());
        // let phys_to_virt = phys_to_virt(PhysAddr::from(phys_base >> 1 << 1));

        // return NonZeroUsize::new_unchecked(phys_to_virt(from).as_usize());
        // return NonZeroUsize::new_unchecked(phys_to_virt.as_usize());

        // let ret = NonZeroUsize::new_unchecked(phys_to_virt.as_usize());
        // info!("return:{:x},byte:{:x}", ret, bytes);
        // return ret;
    }

    fn unmap(&mut self, virt_base: usize, bytes: usize) {}
}

impl XhciController {
    pub fn init(
        pci_bar_address: usize,
        bar_size: usize,
        cap_offset_usize: usize,
    ) -> XhciController {
        // let config_enable = phys_to_virt(PhysAddr::from(0xFA000000));
        // let config_enable: usize = 0x6_0000_0000;
        // unsafe {
        //     info!("writing!");
        //     // while let stat = (*(config_enable.as_usize() as *const u16)) as u16 == 0x10 {
        //     while let stat = (*(config_enable as *const u16)) as u16 == 0x10 {
        //         *((add + 0x04) as *mut u16) = 326;
        //         info!("status:{}", stat);
        //     }
        //     info!("writed!");
        // }

        info!(
            "received address:{:x},offset:{:x},offseted:{:x}",
            pci_bar_address,
            cap_offset_usize,
            pci_bar_address + cap_offset_usize
        );

        let xhci_controller = XhciController {
            controller: Some(unsafe {
                xhci::Registers::new(
                    pci_bar_address,
                    MemoryMapper {
                        // addr_offset: cap_offset_usize,
                    },
                )
            }),
        };

        match &xhci_controller.controller {
            Some(r) => {
                // set_interrupt(r);
                enable_usbs(r);
            }
            None => {}
        }

        xhci_controller
    }
}

fn enable_xhci(reg: Registers<MemoryMapper>) {
    // 获取操作寄存器的引用
    let o = &mut reg.operational;

    // 清除状态位和中断使能位
    o.usbsts
        .write_volatile(UsbStatusRegister(0xFFFFFFFF as u32));
    o.usbcmd.update_volatile(|u| {
        u.clear_interrupter_enable();
    });

    // 为控制器分配设备上下文数组
    let dcbaap = Box::leak(Box::new([0; 256]));
    o.dcbaap.write(dcbaap.as_ptr() as u64);

    // 为控制器分配事件环
    let erst = Box::leak(Box::new([0; 16]));
    o.erstsz.write(1); //TODO: found Event Ring Segment Table Size Register, it's in interrupt reg sets!
    o.erstba.write(erst.as_ptr() as u64);

    // 将事件环的消费者循环索引写入操作寄存器
    o.erdp.write(erst.as_ptr() as u64);

    // 将控制器的运行/停止位置为1，以启动控制器
    o.usbcmd.update_volatile(|u| {
        u.set_run_stop();
    });
    while o.usbsts.read().hc_halted() {}
}

fn enable_usbs(reg: &Registers<MemoryMapper>) {
    // 获取端口的数量
    let port_count = reg.capability.hcsparams1.read().number_of_ports();

    // 遍历每个端口
    for i in 1..port_count {
        info!("enpowering {i}");
        // 获取端口的状态和控制寄存器
        let portsc = &mut reg.port_register_set.read_volatile_at(i as usize).portsc;
        info!("status:{}", portsc.current_connect_status());

        // 检查端口是否连接了设备
        if portsc.current_connect_status() {
            // 重置端口
            portsc.set_port_reset();
            while portsc.port_reset() {
                info!("waiting port reset");
                busy_wait(Duration::from_secs(1));
            }

            // 使能端口
            portsc.set_0_port_enabled_disabled();
            while !portsc.port_enabled_disabled() {
                info!("waiting port enable");
            }
            info!("enabled{:x}", i);

            // 配置端口
            // portsc.update(|p| {
            // 设置端口速度
            // portsc.set_port_speed(PortSpeed::SuperSpeed);
            // 设置端口功率
            portsc.set_port_power();
            // 设置端口链路状态
            portsc.set_port_link_state(0);
            // });

            info!(
                "status of {i}: {},connected?{}",
                portsc.port_link_state(),
                portsc.current_connect_status(),
            )
        }
    }
}

// fn set_interrupt(reg: &Registers<MemoryMapper>) {
//     // 获取中断管理寄存器和中断调节寄存器
//     let iman = &mut reg.runtime.interrupter_register_set[0].iman;
//     let imod = &mut reg.runtime.interrupter_register_set[0].imod;

//     // 启用中断并设置中断间隔为4000微秒和中断计数器为0
//     iman.update(|i| i.set_interrupt_enable());
//     imod.write(InterrupterRegisterSet::new(4000, 0));

//     // 获取USB中断使能寄存器
//     let usbintr = &mut reg.operational.usbintr;

//     // 选择端口状态改变事件和传输完成事件作为中断源
//     usbintr.update(|u| {
//         u.set_port_status_change_event_enable();
//         u.set_transfer_event_enable();
//     });

//     // 获取MSI-X表的地址
//     let msix_table_address = 0xfee00000;

//     // // 获取MSI-X表的指针
//     // let msix_table_ptr = (msix_table_address as *mut u32).as_mut().unwrap();

//     // // 设置中断向量的地址为0xfee00000，数据为0x00000030
//     // msix_table_ptr.write_volatile(0xfee00000);
//     // msix_table_ptr.add(1).write_volatile(0x00000030);

//     // // 定义一个中断处理函数
//     // extern "x86-interrupt" fn xhci_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
//     //     // 读取事件环的生产者循环索引
//     //     let erst = unsafe { &mut *(0x251000 as *mut u32) };
//     //     let producer_cycle_state = erst.read_volatile() & 1;

//     //     // 读取事件环的消费者循环索引
//     //     let erdp = unsafe { &mut *(0x251010 as *mut u32) };
//     //     let consumer_cycle_state = erdp.read_volatile() & 1;

//     //     // 如果生产者和消费者的循环状态相同，说明有新的事件
//     //     if producer_cycle_state == consumer_cycle_state {
//     //         // 读取事件环的当前事件
//     //         let trb = unsafe { &mut *(erdp.read_volatile() as *mut u32) };

//     //         // 根据事件的类型和参数执行相应的操作
//     //         match trb.read_volatile() >> 10 & 0b111111 {
//     //             0b000001 => println!("Transfer Event"),
//     //             0b000100 => println!("Port Status Change Event"),
//     //             _ => println!("Unknown Event"),
//     //         }

//     //         // 更新消费者循环索引和循环状态
//     //         erdp.write_volatile(erdp.read_volatile() + 16);
//     //         erdp.update(|e| e ^ 1);
//     //     }

//     //     // 发送EOI（结束中断）信号
//     //     unsafe {
//     //         PICS.lock().notify_end_of_interrupt(0x30);
//     //     }
//     // }

//     // // 注册中断处理函数到IDT中
//     // idt[0x30].set_handler_fn(xhci_interrupt_handler);
//     //TODO: config interrupt
// }

/// Operations that require a graphics device driver to implement.
pub trait XhciDriverOps: BaseDriverOps {
    /// Get the display information.
    fn info(&self) -> XhciInfo;
}

impl BaseDriverOps for XhciController {
    fn device_name(&self) -> &str {
        //todo  unimplemented!();
        "xhci-controller"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::XHCI
    }
}

impl XhciDriverOps for XhciController {
    fn info(&self) -> XhciInfo {
        todo!()
    }
}
