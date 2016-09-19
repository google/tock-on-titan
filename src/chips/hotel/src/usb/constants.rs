
pub const SOF: u32 = 1 << 3;
pub const EARLY_SUSPEND: u32 = 1 << 10;
pub const USB_SUSPEND: u32 = 1 << 11;
pub const USB_RESET: u32 = 1 << 12;
pub const ENUM_DONE: u32 = 1 << 13;
pub const IEPINT: u32 = 1 << 18;
pub const OEPINT: u32 = 1 << 19;
pub const GOUTNAKEFF: u32 = 1 << 7;
pub const GINNAKEFF: u32 = 1 << 6;

const MAX_CONTROL_ENDPOINTS: u16 = 3;
const MAX_NORMAL_ENDPOINTS: u16 = 16;
pub const MAX_PACKET_SIZE: u16 = 64;
// const FIFO_RAM_DEPTH: u16        = 1024;

pub const RX_FIFO_SIZE: u16 = (4 * MAX_CONTROL_ENDPOINTS + 6) + (2 * (MAX_PACKET_SIZE / 4 + 1)) +
                              (2 * MAX_NORMAL_ENDPOINTS) + 1;
pub const TX_FIFO_SIZE: u16 = 2 * MAX_PACKET_SIZE / 4;
// const ENDPOINT_STATUS_SIZE = 4 * MAX_NORMAL_ENDPOINTS * 2;
