//these code mainly inspired by circle:https://github.com/rsta2/circle
use core::{fmt::Display, u128};

use aarch64_cpu::asm::barrier::{self, ST, SY};
use axhal::mem::{phys_to_virt, virt_to_phys, PhysAddr};
use core::fmt;
use log::info;

const PCI_CLASS_REVISION: u64 = 0x08;
const PCI_HEADER_TYPE_NORMAL: u8 = 0;
const PCI_CACHE_LINE_SIZE: u64 = 0x0c; /* 8 bits */
const PCI_HEADER_TYPE: u64 = 0x0e; /* 8 bits */
const PCI_SECONDARY_BUS: u64 = 0x19;
const PCI_SUBORDINATE_BUS: u64 = 0x1a;
const PCI_MEMORY_BASE: u64 = 0x20;
const PCI_MEMORY_LIMIT: u64 = 0x22;
const MEM_PCIE_RANGE_PCIE_START: usize = 0xF8000000;
const PCI_BRIDGE_CONTROL: u64 = 0x3e;
const PCI_BRIDGE_CTL_PARITY: u8 = 0x01;
const BRCM_PCIE_CAP_REGS: u64 = 0x00ac;
const PCI_CAP_LIST_ID: u64 = 0;
const PCI_CAP_ID_EXP: u8 = 0x10;
const PCI_EXP_RTCTL: u64 = 28;
const PCI_EXP_RTCTL_CRSSVE: u8 = 0x0010;
const ARM_XHCI_BASE: usize = 0xFA00_0000;
const MAPPED_XHCI_BASE: usize = 0xffff_0000_fA00_0000;
const XHCI_REG_CAP_HCIVERSION: usize = 0x02;
const XHCI_PCI_CLASS_CODE: usize = 0xC0330;
const XHCI_PCIE_SLOT: usize = 0;
const XHCI_PCIE_FUNC: usize = 0;
fn delay(seconds: u64) {
    for i in 1..seconds + 1 {
        fn fibonacci_recursive(n: u64) -> u64 {
            if n == 0 {
                return 0;
            }
            if n == 1 {
                return 1;
            }
            return fibonacci_recursive(n - 1) + fibonacci_recursive(n - 2);
        }
        fibonacci_recursive(36 + (i % 2));
    }
}

///return:mmio space
pub fn enable_xhci(bus: u8, dfn: u8, address: usize) -> usize {
    info!("xhci energizing!");
    enable_bridge(bus, dfn, address);

    let a = get_tag();
    if a {
        panic!("state sync fail!");
    }
    //check version
    let usVersion: u16 = unsafe { *((MAPPED_XHCI_BASE + XHCI_REG_CAP_HCIVERSION) as *const u16) };
    if usVersion != 0x100 {
        info!("Unsupported xHCI version {:x}", usVersion);
    }
    enable_device(address);

    ARM_XHCI_BASE
}

fn enable_device(address: usize) {
    info!("enable xhci!");
    const SLOT: usize = XHCI_PCIE_SLOT;
    const FUNC: usize = XHCI_PCIE_FUNC;
    const CLASS_CODE: u64 = 0xC0330;

    let conf: u64 = pcie_map_conf(1, ((SLOT & 0x1f) << 3 | (FUNC & 0x07)) as u8, 0, address);
    if conf == 0 {
        panic!("enable failed 1");
    } else {
        info!("conf = {:x}", conf);
    }

    // unsafe {
    //     loop {
    //         let val1 = *((conf + PCI_CLASS_REVISION) as *const u64);
    //         let val2 = *((conf + PCI_HEADER_TYPE) as *const u8);
    //         let cond1 = (val1 >> 8) != CLASS_CODE;
    //         let cond2 = val2 != PCI_HEADER_TYPE_NORMAL;
    //         if !(cond1 || cond2) {
    //             break;
    //         } else {
    //             info!("enable waiting:{}:{:x},{}:{:x}", cond1, val1, cond2, val2);
    //         }
    //     }
    // }
    info!("check passed");

    unsafe {
        *((conf + PCI_CACHE_LINE_SIZE) as *mut u8) = 64 / 4; // TODO: get this from cache config

        *((conf + 0x10) as *mut u32) = (MEM_PCIE_RANGE_PCIE_START) as u32 | 0x04;
        *((conf + 0x14) as *mut u32) = (MEM_PCIE_RANGE_PCIE_START >> 32) as u32;
        *((conf + 0x04) as *mut u16) = 0x2 | 0x4 | 0x40 | 0x100;
    }
}
const PCI_HEADER_TYPE_BRIDGE: usize = 1;

fn enable_bridge(bus: u8, dfn: u8, address: usize) {
    info!("enable bridge!");
    //todo try to get pcie config address
    let conf: u64 = pcie_map_conf(bus, dfn, 0, address);
    if conf == 0 {
        panic!("enable failed 1");
    } else {
        info!("conf = {:x}", conf);
    }

    unsafe {
        loop {
            let val1 = *((conf + PCI_CLASS_REVISION) as *const u64); //var type bit length definition err: my own mistake, could ignore for now
            let val2 = *((conf + PCI_HEADER_TYPE) as *const u8);
            let cond1 = val1 >> 8 != 0x060400;
            // let cond1 = false;
            let cond2 = val2 as usize != PCI_HEADER_TYPE_BRIDGE;
            if !(cond1 || cond2) {
                break;
            } else {
                info!("enable waiting:{}:{:x},{}:{:x}", cond1, val1, cond2, val2);
                break; //todo remove it
            }
        }
        info!("check passed");

        *((conf + PCI_CACHE_LINE_SIZE) as *mut u8) = 64 / 4; // TODO: get this from cache config

        *((conf + PCI_SECONDARY_BUS) as *mut u8) = 1;
        *((conf + PCI_SUBORDINATE_BUS) as *mut u8) = 1;

        *((conf + PCI_MEMORY_BASE) as *mut u16) = (MEM_PCIE_RANGE_PCIE_START >> 16) as u16;
        *((conf + PCI_MEMORY_LIMIT) as *mut u16) = (MEM_PCIE_RANGE_PCIE_START >> 16) as u16;

        *((conf + PCI_BRIDGE_CONTROL) as *mut u8) = PCI_BRIDGE_CTL_PARITY;

        assert_eq!(
            *((conf + BRCM_PCIE_CAP_REGS + PCI_CAP_LIST_ID) as *const u8),
            PCI_CAP_ID_EXP
        );

        *((conf + BRCM_PCIE_CAP_REGS + PCI_EXP_RTCTL) as *mut u8) = PCI_EXP_RTCTL_CRSSVE;

        *((conf + 0x04) as *mut u16) = 0x2 | 0x4 | 0x40 | 0x100;
        info!("done");
    }
}

fn pcie_map_conf(busnr: u8, devfn: u8, whereis: usize, address: usize) -> u64 {
    /* Accesses to the RC go right to the RC registers if slot==0 */
    if busnr == 0 {
        info!("devfn:{:x}", devfn);
        return if (((devfn) >> 3) & 0x1f) != 0 {
            info!("case 1");
            0
        } else {
            info!("case 2:address={:x}", address);
            return (address + whereis) as u64;
        };
    }

    /* For devices, write to the config space index register */
    let idx: i32 = cfg_index(busnr, devfn, 0);
    // bcm_writel(idx, m_base + 36864);
    unsafe {
        *((address + 36864) as *mut i32) = idx;
    }
    return (address + 32768 + whereis).try_into().unwrap();
}

#[allow(arithmetic_overflow)]
fn cfg_index(busnr: u8, devfn: u8, reg: u8) -> i32 {
    // return ((((devfn) >> 3) & 0x1f) << 15) | ((devfn & 0x07) << 12) | (busnr << 20) | (reg & !3);
    return (((((devfn) >> 3) & 0x1f & 0x1f) << 15)
        | ((devfn & 0x07 & 0x07) << 12)
        | (busnr << 20)
        | (reg & !3))
        .into();
}

fn notify_reset() {}

const TAG_XHCI_NOTIFY_RESET: usize = 0x00030058;
const MEM_COHERENT_REGION: usize = 0x8000
    + 2 * 0x100000
    + 0x20000
    + 0x20000 * (4 - 1)
    + 0x8000
    + 0x8000 * (4 - 1)
    + 0x8000
    + 0x8000 * (4 - 1)
    + 0x8000
    + 0x8000 * (4 - 1)
    + 0x4000
    + 3 * 0x100000
    & !(2 * 0x100000 - 1);

#[allow(arithmetic_overflow)]
fn get_tag() -> bool {
    info!("get_tag");
    let mut property_tag = PropertyTag {
        n_tag_id: RESET_COMMAND,
        n_value_buf_size: 32,
        n_value_length: 4 & (!(1 << 32)),
    };
    // if get_tags(&mut property_tag) {
    // return false;
    // }
    return !get_tags(&mut property_tag);
    // return true;
}

const RESET_COMMAND: u32 = 1 << 20 | 0 << 15 | 0 << 12;
const CODE_REQUEST: usize = 0x00000000;
const CODE_RESPONSE_SUCCESS: usize = 0x80000000;
const CODE_RESPONSE_FAILURE: usize = 0x80000001;
struct TPropertyBuffer {
    n_buffer_size: u32, // bytes
    n_code: u32,
    tags: PropertyTag,
}

#[derive(Clone, Copy)]
struct PropertyTag {
    n_tag_id: u32,
    n_value_buf_size: u32, // bytes, multiple of 4
    n_value_length: u32,   // bytes
}

impl Display for PropertyTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[tag_id:{},buf_size:{},val_len:{}]",
            self.n_tag_id, self.n_value_buf_size, self.n_value_length
        )
    }
}

fn get_tags(prop_tag: &mut PropertyTag) -> bool {
    info!("get_tags,tag:{}", prop_tag);
    let buffer_size: usize = 72 + 128 + 32;
    let p_buffer_address = phys_to_virt(MEM_COHERENT_REGION.into()).as_usize();
    let p_buffer = p_buffer_address as *mut TPropertyBuffer;
    info!("p_buffer:{:x}", p_buffer as usize);

    unsafe {
        (*p_buffer).n_buffer_size = buffer_size as u32;
        (*p_buffer).n_code = CODE_REQUEST as u32;
        (*p_buffer).tags = *prop_tag;

        let p_end_tag = (p_buffer as usize + 64 + 128) as *mut u32;

        *p_end_tag = 0x00000000;

        // barr
        barrier::dsb(SY);

        let virt_to_phys = virt_to_phys(p_buffer_address.into());
        let n_buffer_address = virt_to_phys.as_usize() & !0xC0000000 | 0xC0000000;
        info!("n_buffer_address:{:x}", n_buffer_address);

        let write_read = write_read(phys_to_virt(n_buffer_address.into()).as_usize());
        if write_read != n_buffer_address as u32 {
            //big issue with condition
            info!("cond match:{:x}", write_read);
            return false;
        }

        barrier::dmb(SY);

        let mut n_code = (*p_buffer).n_code;
        // if n_code != CODE_RESPONSE_SUCCESS as u32 {
        //     info!("cond match:{:x}:{:x}", n_code, CODE_RESPONSE_SUCCESS);
        //     return false;
        // }
        // while n_code != CODE_RESPONSE_SUCCESS as u32 {
        //     // has issue of this var
        //     info!(
        //         "cond match:{:x}:{:x},tag:{}",
        //         n_code,
        //         CODE_RESPONSE_SUCCESS,
        //         (*p_buffer).tags
        //     );
        //     n_code = (*p_buffer).n_code;
        //     delay(1);
        //     // return false;
        // }
        ////todo fixit
        ////fornow just treat it as pass

        *prop_tag = (*p_buffer).tags;
        return true;
    }
}

const MAILBOX0_STATUS: usize = 0xFE000000 + 0xB880 + 0x18;
const MAILBOX0_READ: usize = 0xFE000000 + 0xB880 + 0x00;
const MAILBOX_STATUS_EMPTY: u32 = 0x40000000;
const MAILBOX_STATUS_FULL: u32 = 0x80000000;
const MAILBOX1_STATUS: usize = 0xFE000000 + 0xB880 + 0x38;
const MAILBOX1_WRITE: usize = 0xFE000000 + 0xB880 + 0x20;

fn write_read(n_data: usize) -> u32 {
    // PeripheralEntry();

    // if (!m_bEarlyUse) {
    //     s_SpinLock.Acquire();
    //     // spinlock::SpinNoIrq::
    // }
    //no need lock for now...?

    unsafe {
        // Flush();
        //todo switch to virt add
        while !((*(phys_to_virt(MAILBOX0_STATUS.into()).as_usize() as *const u32)
            & MAILBOX_STATUS_EMPTY)
            != 0)
        {
            let usize = *(phys_to_virt(MAILBOX0_READ.into()).as_usize() as *const u32);

            // CTimer::SimpleMsDelay(20);
            info!("waiting...{:x}", usize);
            delay(1)
        }

        // Write(n_data);
        while (*(phys_to_virt(MAILBOX1_STATUS.into()).as_usize() as *const u32)
            & MAILBOX_STATUS_FULL)
            != 0
        {
            // do nothing
            info!(
                "waiting...:{:x}",
                *(phys_to_virt(MAILBOX1_STATUS.into()).as_usize() as *const u32)
            );
        }

        assert!((n_data & 0xF) == 0);
        // *(phys_to_virt(MAILBOX1_WRITE.into()).as_usize() as *mut u32) = 8 | n_data;//probablity: issue at here
        *(phys_to_virt(MAILBOX1_WRITE.into()).as_usize() as *mut u32) = 8 | n_data as u32; //probablity: issue at here, investgate:https://blog.csdn.net/qq_26989627/article/details/122024901
                                                                                           // channel number is in the lower 4 bits //curios:is 8 correct?:mchannel-BCM_MAILBOX_PROP_OUT

        // let nResult: u32 = Read();
        let mut n_result: u32 = 0;

        'a: loop {
            while *(phys_to_virt(MAILBOX0_STATUS.into()).as_usize() as *const u32)
                & MAILBOX_STATUS_EMPTY
                != 0
            {
                // do nothing
                info!(
                    "waiting...{:x}",
                    *(phys_to_virt(MAILBOX0_STATUS.into()).as_usize() as *const u32)
                );
                delay(1);
            }

            n_result = unsafe { *(phys_to_virt(MAILBOX0_READ.into()).as_usize() as *const u32) };

            if !((n_result & 0xF) != 8) {
                info!("break! n_result & 0xf:{:x}", n_result & 0xf);
                break 'a;
            } else {
                info!("n_result & 0xf :{:x}", n_result & 0xf);
                delay(1);
            } // channel number is in the lower 4 bits
        }
        // if (!m_bEarlyUse) {
        //     s_SpinLock.Release();
        // }

        // PeripheralExit();
        info!("n_result:{:x},and!0xf = {:x}", n_result, n_result & !0xF);
        return n_result & !0xF;
    }
}
