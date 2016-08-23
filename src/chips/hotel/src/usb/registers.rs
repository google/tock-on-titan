use common::volatile_cell::VolatileCell;

pub struct Registers {
    pub otg_control: VolatileCell<u32>,
    pub otg_interrupt: VolatileCell<u32>,
    pub ahb_config: VolatileCell<u32>,
    pub configuration: VolatileCell<u32>,
    pub reset: VolatileCell<u32>,
    pub interrupt_status: VolatileCell<u32>,
    pub interrupt_mask: VolatileCell<u32>,
    pub _grxstsr: VolatileCell<u32>,
    pub _grxstsp: VolatileCell<u32>,
    pub receive_fifo_size: VolatileCell<u32>,
    pub transmit_fifo_size: VolatileCell<u32>,

    _reserved: [u32; 3],
    //0x38
    /// The `gpio` register is a portal to a set of custom 8-bit registers.
    ///
    /// Logically it is split into a GP_OUT part and a GP_IN part. Writing to a
    /// custom register can be done in a single operation, with all data
    /// transferred in GP_OUT. Reading requires a GP_OUT write to select the
    /// register to read, then a read or GP_IN to see what the register holds.
    ///   GP_OUT:
    ///    bit  15     direction: 1=write, 0=read
    ///    bits 11:4   value to write to register when bit 15 is set
    ///    bits 3:0    custom register to access
    ///   GP_IN:
    ///    bits 7:0    value read back from register when GP_OUT[15] is clear
    pub gpio: VolatileCell<u32>,
    pub guid: VolatileCell<u32>,
    pub gsnpsid: VolatileCell<u32>,
    pub user_hw_config: [VolatileCell<u32>; 4],

    _reserved0: [u32; 2],

    pub gdfifocfg: VolatileCell<u32>,

    _reserved1: [u32; 41],

    pub device_in_ep_tx_fifo_size: [VolatileCell<u32>; 15],

    _reserved2: [u32; 432],

    pub device_config: VolatileCell<u32>,
    pub device_control: VolatileCell<u32>,
    pub device_status: VolatileCell<u32>,

    _reserved_3: u32,
    // 0x810
    pub device_in_ep_interrupt_mask: VolatileCell<u32>,
    pub device_out_ep_interrupt_mask: VolatileCell<u32>,
    pub device_all_ep_interrupt: VolatileCell<u32>,
    pub device_all_ep_interrupt_mask: VolatileCell<u32>,

    _reserved_4: [u32; 2],
    //0x828
    pub device_vbus_discharge_time: VolatileCell<u32>,
    pub device_vbus_pulsing_time: VolatileCell<u32>,
    pub device_threshold_control: VolatileCell<u32>,
    pub device_in_ep_fifo_empty_interrupt_mask: VolatileCell<u32>,

    _reserved_5: [u32; 50],
    //0x900
    pub in_endpoints: [InEndpoint; 16],
    pub out_endpoints: [OutEndpoint; 16],
    //0xd00
    _reserved6: [u32; 64],
    //0xe00
    pub power_clock_gating_control: VolatileCell<u32>
}

pub struct InEndpoint {
    pub control: VolatileCell<u32>,
    _reserved0: u32,
    pub interrupt: VolatileCell<u32>,
    _reserved1: u32,
    pub transfer_size: VolatileCell<u32>,
    pub dma_address: VolatileCell<u32>,
    pub tx_fifo_status: VolatileCell<u32>,
    pub buffer_address: VolatileCell<u32>
}

pub struct OutEndpoint {
    pub control: VolatileCell<u32>,
    _reserved0: u32,
    pub interrupt: VolatileCell<u32>,
    _reserved1: u32,
    pub transfer_size: VolatileCell<u32>,
    pub dma_address: VolatileCell<u32>,
    _reserved2: u32,
    pub buffer_address: VolatileCell<u32>
}

pub struct DMADescriptor {
    pub flags: u32,
    pub addr: usize
}

