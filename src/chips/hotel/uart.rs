use common::volatile_cell::VolatileCell;

pub struct Registers {
    pub read_data: VolatileCell<u32>,
    pub write_data: VolatileCell<u32>,
    pub nco: VolatileCell<u32>,
    pub control: VolatileCell<u32>,
    pub interrupt_control: VolatileCell<u32>,
    pub state: VolatileCell<u32>,
    pub clear_state: VolatileCell<u32>
}

pub const UART0 : *mut Registers = 0x40600000 as *mut Registers;
pub const UART1 : *mut Registers = 0x40610000 as *mut Registers;
pub const UART2 : *mut Registers = 0x40620000 as *mut Registers;

