// 引入xhci库
//TODO 迁移到xhci::ring::trb
use xhci::Registers;

// 定义CommandRing结构体
pub struct CommandRing {
    // 命令环的指针
    ring_ptr: *mut u64,
    // 命令环的大小
    ring_size: usize,
    // 生产者循环索引
    cycle_state: u32,
}

// 实现CommandRing结构体的方法
impl CommandRing {
    // 创建一个新的命令环
    pub fn new(ring_ptr: *mut u64) -> Self {
        // 为命令环分配内存
        let ring_ptr = ring_ptr;
        // 设置命令环的大小为16
        let ring_size = 16;
        // 设置生产者循环索引为0
        let cycle_state = 0;
        // 返回一个CommandRing的实例
        CommandRing {
            ring_ptr,
            ring_size,
            cycle_state,
        }
    }

    // 获取命令环的地址
    pub fn address(&self) -> usize {
        // 返回命令环的指针转换为usize
        self.ring_ptr as usize
    }

    // 创建一个命令TRB
    pub fn create_command_trb<F>(&mut self, f: F) -> *mut u64
    where
        F: FnOnce(&mut CommandTrb),
    {
        // 获取命令环的当前位置
        let current_ptr = unsafe { self.ring_ptr.add(self.cycle_state as usize) };
        // 获取命令TRB的引用
        let trb = unsafe { &mut *(current_ptr as *mut CommandTrb) };
        // 调用闭包来设置命令TRB的字段
        f(trb);
        // 返回命令TRB的指针
        current_ptr
    }

    // 将一个命令TRB加入命令环
    pub fn push_command_trb(&mut self, trb_ptr: *mut u64) {
        // 获取命令TRB的引用
        let trb = unsafe { &mut *(trb_ptr as *mut CommandTrb) };
        // 设置命令TRB的循环位为当前的循环状态
        trb.set_cycle_bit(self.cycle_state);
        // 更新生产者循环索引
        self.cycle_state = (self.cycle_state + 1) % self.ring_size as u32;
    }
}

// 定义CommandType结构体
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum CommandType {
    // 使能中断
    EnableInterrupt = 0b000001,
    // 禁用中断
    DisableInterrupt = 0b000010,
    // 设置端口功率
    SetPortPower = 0b000011,
    // 重置端口
    ResetPort = 0b000100,
    // 配置端点
    ConfigureEndpoint = 0b000101,
    // 评估上下文
    EvaluateContext = 0b000110,
    // 重置端点
    ResetEndpoint = 0b000111,
    // 停止端点
    StopEndpoint = 0b001000,
    // 设置端点委托状态
    SetEndpointDequeueState = 0b001001,
    // 重置设备
    ResetDevice = 0b001010,
    // 其他命令类型
    // ...
}

// 实现CommandType结构体的方法
impl CommandType {
    // 将命令类型转换为u32
    pub fn to_u32(&self) -> u32 {
        // 将命令类型转换为u8
        let value = (*self) as u8;
        // 将命令类型左移10位，以对齐TRB的字段
        (value as u32) << 10
    }
}

// 定义CommandTrb结构体
#[repr(C)]
pub struct CommandTrb {
    // 参数1
    parameter1: u32,
    // 参数2
    parameter2: u32,
    // 状态和控制
    status_and_control: u32,
    // 命令类型
    command_type: u32,
}

// 实现CommandTrb结构体的方法
impl CommandTrb {
    // 设置参数1
    pub fn set_parameter1(&mut self, value: u32) {
        self.parameter1 = value;
    }

    // 设置参数2
    pub fn set_parameter2(&mut self, value: u32) {
        self.parameter2 = value;
    }

    // 设置状态和控制
    pub fn set_status_and_control(&mut self, value: u32) {
        self.status_and_control = value;
    }

    // 设置命令类型
    pub fn set_type(&mut self, value: CommandType) {
        self.command_type = value.to_u32();
    }

    // 设置中断目标
    pub fn set_interrupt_target(&mut self, value: u8) {
        self.status_and_control |= (value as u32) << 22;
    }

    // 设置循环位
    pub fn set_cycle_bit(&mut self, value: u32) {
        self.status_and_control |= value & 1;
    }
}
