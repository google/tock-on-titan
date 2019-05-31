// Copyright 2018 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(dead_code)]

// The USB stack currently expects 7 strings, at these indices.
pub const STRING_LANG: u8       = 0;
pub const STRING_VENDOR: u8     = 1;
pub const STRING_BOARD: u8      = 2;
pub const STRING_PLATFORM: u8   = 3;
pub const STRING_INTERFACE1: u8 = 4;  // Shell
pub const STRING_BLAH: u8       = 5;  // Garbage?
pub const STRING_INTERFACE2: u8 = 6;  // Hotel_U2F

const MAX_CONTROL_ENDPOINTS: u16 =  3;
const MAX_NORMAL_ENDPOINTS:  u16 = 16;
pub const MAX_PACKET_SIZE:   u16 = 64;
pub const U2F_REPORT_SIZE:   u16 = 64;

// Constants defining buffer sizes for all endpoints.
pub const EP_BUFFER_SIZE_BYTES:    usize = MAX_PACKET_SIZE as usize;
pub const EP_BUFFER_SIZE_WORDS:    usize = EP_BUFFER_SIZE_BYTES / 4;

// Complicated FIFP size formula specified in reference manual
pub const RX_FIFO_SIZE: u16 = (4 * MAX_CONTROL_ENDPOINTS + 6) +
                              (2 * (MAX_PACKET_SIZE / 4 + 1)) +
                              (2 * MAX_NORMAL_ENDPOINTS) + 1;
pub const TX_FIFO_SIZE: u16 = 2 * MAX_PACKET_SIZE / 4;


#[repr(u32)]
pub enum Configuration {
    // Timing values copied from Cr50 C reference code
    TimeoutCalibration7 = 7  <<  0,
    Unidirectional6Pin  = 0  <<  5,
    FullSpeed1_1        = 1  <<  6,
    TurnaroundTime14    = 14 << 10,
}

#[repr(u32)]
pub enum Gpio {
    PhyA       = 0b100 << 4,
    PhyB       = 0b101 << 4,
    WriteMode  = 1 << 15,
}

#[derive(PartialEq)]
#[repr(u32)]
pub enum Interrupt {
    HostMode           = 1 <<  0,
    Mismatch           = 1 <<  1,
    OTG                = 1 <<  2,
    SOF                = 1 <<  3,
    RxFIFO             = 1 <<  4,
    GlobalInNak        = 1 <<  6,
    GlobalOutNak       = 1 <<  7,
    EarlySuspend       = 1 << 10,
    Suspend            = 1 << 11,
    Reset              = 1 << 12,
    EnumDone           = 1 << 13,
    OutISOCDrop        = 1 << 14,
    EOPF               = 1 << 15,
    EndpointMismatch   = 1 << 17,
    InEndpoints        = 1 << 18,
    OutEndpoints       = 1 << 19,
    InISOCIncomplete   = 1 << 20,
    IncompletePeriodic = 1 << 21,
    FetchSuspend       = 1 << 22,
    ResetDetected      = 1 << 23,
    ConnectIDChange    = 1 << 28,
    SessionRequest     = 1 << 30,
    ResumeWakeup       = 1 << 31,
}

#[allow(dead_code)]
pub enum Reset {
    CSftRst          =  1 <<  0,
    RxFFlsh          =  1 <<  4,
    TxFFlsh          =  1 <<  5,
    FlushFifo0       =  0 <<  6,
    FlushFifo1       =  1 <<  6,
    FlushFifo2       =  2 <<  6,
    FlushFifo3       =  3 <<  6,
    FlushFifo4       =  4 <<  6,
    FlushFifo5       =  5 <<  6,
    FlushFifo6       =  6 <<  6,
    FlushFifo7       =  7 <<  6,
    FlushFifo8       =  8 <<  6,
    FlushFifo9       =  9 <<  6,
    FlushFifo10      = 10 <<  6,
    FlushFifo11      = 11 <<  6,
    FlushFifo12      = 12 <<  6,
    FlushFifo13      = 13 <<  6,
    FlushFifo14      = 14 <<  6,
    FlushFifo15      = 15 <<  6,
    FlushFifoAll     = 16 <<  6, // It's 5 bits, 0x10 means all FIFOs
    DMAReq           = 1 << 30,
    AHBIdle          = 1 << 31,
}

#[allow(dead_code)]
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

// OTG Databook, Table 5-58
#[allow(dead_code)]
pub enum OutInterrupt {
    XferComplete =      1 <<  0,
    EPDisabled =        1 <<  1,
    AHBErr =            1 <<  2,
    SetUP =             1 <<  3,
    OutToknEPdis =      1 <<  4,
    StsPhseRcvd =       1 <<  5,
    Back2BackSETup =    1 <<  6,
    // Bit 7 reserved
    OutPkrErr =         1 <<  8,
    BnaOutIntr =        1 <<  9,
    // Bits 10-11 reserved
    BbleErr =           1 << 12,
    NAK =               1 << 13,
    NYET =              1 << 14,
    // Bits 15-31 reserved
}

// OTG Databook, Table 5-76
#[allow(dead_code)]
pub enum InInterrupt {
    XferComplete =         1 << 0,
    EPDisabled   =         1 << 1,
    AHBErr       =         1 << 2,
    Timeout      =         1 << 3,
    InTokenRecv  =         1 << 4,
    InTokenEPMis =         1 << 5,
    InNakEffect  =         1 << 6,
    TxFifoReady  =         1 << 7,
    TxFifoUnder  =         1 << 8,
    BuffNotAvail =         1 << 9,
    PacketDrop   =         1 << 11,
    BabbleErr    =         1 << 12,
    NAK          =         1 << 13,
    NYET         =         1 << 14,
    SetupRecvd   =         1 << 15,
}

// OTG Databook, Table 5-9
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum AhbConfig {
    GlobalInterruptMask         = 1 <<  0,
    BurstLen1Word               = 0b0000 << 1,
    BurstLen4Words              = 0b0001 << 1,
    BurstLen8Words              = 0b0010 << 1,
    BurstLen16Words             = 0b0011 << 1,
    BurstLen32Words             = 0b0100 << 1,
    BurstLen64Words             = 0b0101 << 1,
    BurstLen128Words            = 0b0110 << 1,
    BurstLen256Words            = 0b0111 << 1,
    DmaEnable                   = 1 <<  5,
    NonPeriodicTxFifoEmptyLevel = 1 <<  7,
    PeriodicTxFifoEmptyLevel    = 1 <<  8,
    RemoteMemorySupport         = 1 << 21,
    NotifyAllDmaWrite           = 1 << 22,
    AhbSingleSupport            = 1 << 23,
    InverseDescEndianness       = 1 << 24,
}
// OTG Databook, Table 5-53
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum DeviceConfig {
//    DeviceSpeedHigh                    = 0b00 << 0,
    DeviceSpeedFull2                   = 0b01 << 0, // 2.0 clock 30 or 60 MHz
    DeviceSpeedLow                     = 0b10 << 0,
    DeviceSpeedFull1                   = 0b11 << 0, // 1.1 clock 48Mhz
//    NonZeroLengthStatusOutHandshake    = 1 <<  2,
    Enable32KHzSuspendMode             = 1 <<  3,

    DeviceAddressShift            =  4,
    DeviceAddressWidth            =  7,

//    PeriodicFrameInterval80            = 0b00 << 11,
    PeriodicFrameInterval85            = 0b01 << 11,
    PeriodicFrameInterval90            = 0b10 << 11,
    PeriodicFrameInterval95            = 0b11 << 11,
    EnableDeviceOutNak                 = 1 << 13,
    XcvrDelay                          = 1 << 14,
    ErraticErrorInterruptMask          = 1 << 15,

    InEndpointMismatchCountShift  = 18,
    InEndpointMismatchCountWidth  =  5,

    EnableScatterGatherDMAInDeviceMode = 1 << 23,
//    PeriodicSchedulingInterval25       = 0b00 << 24,
    PeriodicSchedulingInterval50       = 0b01 << 24,
    PeriodicSchedulingInterval75       = 0b10 << 24,

    ResValidShift                 = 26,
    ResValidWidth                 =  6,

}

// OTG Databook, Table 5-54
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum DeviceControl {
    RemoteWakeupSignaling        = 1 <<  0,
    SoftDisconnect               = 1 <<  1,
    GlobalNonPeriodicInNakStatus = 1 <<  2,
    GlobalOutNakStatus           = 1 <<  3,
    TestControlDisabled          = 0b000 <<  4,
    TestControlModeJ             = 0b001 <<  4,
    TestControlModeK             = 0b010 <<  4,
    TestControlModeSE0Nak        = 0b011 <<  4,
    TestControlModePacket        = 0b100 <<  4,
    TestControlModeForceEnable   = 0b101 <<  4,
    SetGlobalNonPeriodicInNak    = 1 <<  7,
    ClearGlobalNonPeriodicInNak  = 1 <<  8,
    SetGlobalOutNak              = 1 <<  9,
    ClearGlobalOutNak            = 1 << 10,
    PowerOnProgrammingDone       = 1 << 11,
//    GlobalMultiCountInvalid      = 0b00 << 13,
    GlobalMultiCount1Packet      = 0b01 << 13,
    GlobalMultiCount2Packets     = 0b10 << 13,
    GlobalMultiCount3Packets     = 0b11 << 13,
    IgnoreFrameNumber            = 1 << 15,
    NakOnBabbleError             = 1 << 16,
    EnableContinueOnBna          = 1 << 17,
    DeepSleepBESLReject          = 1 << 18,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum Descriptor {
    Device          = 0x01,
    Configuration   = 0x02,
    String          = 0x03,
    Interface       = 0x04,
    Endpoint        = 0x05,
    DeviceQualifier = 0x06,
    HidDevice       = 0x21,
    Report          = 0x22,
    Unknown         = 0xFF,
}

impl Descriptor {
    pub fn from_u8(t: u8) -> Descriptor {
        match t {
            0x01 => Descriptor::Device,
            0x02 => Descriptor::Configuration,
            0x03 => Descriptor::String,
            0x04 => Descriptor::Interface,
            0x05 => Descriptor::Endpoint,
            0x06 => Descriptor::Endpoint,
            0x21 => Descriptor::HidDevice,
            0x22 => Descriptor::Report,
            _    => Descriptor::Unknown,
        }
    }
}

#[allow(dead_code)]
pub const GET_DESCRIPTOR_DEVICE: u32           = 1;
pub const GET_DESCRIPTOR_CONFIGURATION: u32    = 2;
pub const GET_DESCRIPTOR_STRING: u32           = 3;
pub const GET_DESCRIPTOR_INTERFACE: u32        = 4;
pub const GET_DESCRIPTOR_ENDPOINT: u32         = 5;
pub const GET_DESCRIPTOR_DEVICE_QUALIFIER: u32 = 6;
pub const GET_DESCRIPTOR_DEBUG: u32            = 10;

// Copied from Cr52 usb_hidu2f.c - pal
pub const U2F_REPORT_DESCRIPTOR: [u8; 34] = [
    0x06, 0xD0, 0xF1, /* Usage Page (FIDO Alliance), FIDO_USAGE_PAGE */
    0x09, 0x01,       /* Usage (U2F HID Authenticator Device),
                         FIDO_USAGE_U2FHID */
    0xA1, 0x01,       /* Collection (Application), HID_APPLICATION */
    0x09, 0x20,       /*   Usage (Input Report Data), FIDO_USAGE_DATA_IN */
    0x15, 0x00,       /*   Logical Minimum (0) */
    0x26, 0xFF, 0x00, /*   Logical Maximum (255) */
    0x75, 0x08,       /*   Report Size (8) */
    0x95, 0x40,       /*   Report Count (64), HID_INPUT_REPORT_BYTES */
    0x81, 0x02,       /*   Input (Data, Var, Abs), Usage */
    0x09, 0x21,       /*   Usage (Output Report Data), FIDO_USAGE_DATA_OUT */
    0x15, 0x00,       /*   Logical Minimum (0) */
    0x26, 0xFF, 0x00, /*   Logical Maximum (255) */
    0x75, 0x08,       /*   Report Size (8) */
    0x95, 0x40,       /*   Report Count (64), HID_OUTPUT_REPORT_BYTES */
    0x91, 0x02,       /*   Output (Data, Var, Abs), Usage */
    0xC0              /* End Collection */
];

pub enum U2fHidCommand {
    Error = 0xbf,
}
