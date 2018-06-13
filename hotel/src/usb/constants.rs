
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

// Ask Amit 
pub const RX_FIFO_SIZE: u16 = (4 * MAX_CONTROL_ENDPOINTS + 6) + (2 * (MAX_PACKET_SIZE / 4 + 1)) +
                              (2 * MAX_NORMAL_ENDPOINTS) + 1;
pub const TX_FIFO_SIZE: u16 = 2 * MAX_PACKET_SIZE / 4;
// const ENDPOINT_STATUS_SIZE = 4 * MAX_NORMAL_ENDPOINTS * 2;

#[derive(PartialEq)]
pub enum Interrupt {
    HostMode         = 1 <<  0,
    Mismatch         = 1 <<  1,
    OTG              = 1 <<  2,
    SOF              = 1 <<  3,
    RxFIFO           = 1 <<  4,
    GlobalInNak      = 1 <<  6,
    OutNak           = 1 <<  7,
    EarlySuspend     = 1 << 10,
    Suspend          = 1 << 11,
    Reset            = 1 << 12,
    EnumDone         = 1 << 13,
    OutISOCDrop      = 1 << 14,
    EOPF             = 1 << 15,
    EndpointMismatch = 1 << 17,
    InEndpoints      = 1 << 18,
    OutEndpoints     = 1 << 19,
    InISOCIncomplete = 1 << 20,
    IncompletePeriodic = 1 << 21,
    FetchSuspend       = 1 << 22,
    ResetDetected      = 1 << 23,
    ConnectIDChange    = 1 << 28,
    SessionRequest     = 1 << 30,
    ResumeWakeup      = 1 << 31,
}

pub enum Reset {
    CSftRst          = 1 <<  0,
    RxFFlsh          = 1 <<  4,
    TxFFlsh          = 1 <<  5,
    DMAReq           = 1 << 30,
    AHBIdle          = 1 << 31,
}

pub enum AllEndpointInterruptMask {
    IN0   = 1 <<  0,
    IN1   = 1 <<  1,
    IN2   = 1 <<  2,
    IN3   = 1 <<  3,
    IN4   = 1 <<  4,
    IN5   = 1 <<  5,
    IN6   = 1 <<  6,
    IN7   = 1 <<  7,
    IN8   = 1 <<  8,
    IN9   = 1 <<  9,
    IN10  = 1 << 10,
    IN11  = 1 << 11,
    IN12  = 1 << 12,
    IN13  = 1 << 13,
    IN14  = 1 << 14,
    IN15  = 1 << 15,
    OUT0  = 1 << 16,
    OUT1  = 1 << 17,
    OUT2  = 1 << 18,
    OUT3  = 1 << 19,
    OUT4  = 1 << 20,
    OUT5  = 1 << 21,
    OUT6  = 1 << 22,
    OUT7  = 1 << 23,
    OUT8  = 1 << 24,
    OUT9  = 1 << 25,
    OUT10 = 1 << 26,
    OUT11 = 1 << 27,
    OUT12 = 1 << 28,
    OUT13 = 1 << 29,
    OUT14 = 1 << 30,
    OUT15 = 1 << 31,
}

pub const GET_DESCRIPTOR_DEVICE: u32           = 1;
pub const GET_DESCRIPTOR_CONFIGURATION: u32    = 2;
pub const GET_DESCRIPTOR_DEVICE_QUALIFIER: u32 = 6;

    
