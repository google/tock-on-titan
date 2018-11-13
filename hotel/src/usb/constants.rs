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
pub const STRING_INTERFACE2: u8 = 6;  // Haven_U2F


pub const SOF: u32           = 1 << 3;
pub const EARLY_SUSPEND: u32 = 1 << 10;
pub const USB_SUSPEND: u32   = 1 << 11;
pub const USB_RESET: u32     = 1 << 12;
pub const ENUM_DONE: u32     = 1 << 13;
pub const IEPINT: u32        = 1 << 18;
pub const OEPINT: u32        = 1 << 19;
pub const GOUTNAKEFF: u32    = 1 << 7;
pub const GINNAKEFF: u32     = 1 << 6;

const MAX_CONTROL_ENDPOINTS: u16 = 3;
const MAX_NORMAL_ENDPOINTS: u16 = 16;
pub const MAX_PACKET_SIZE: u16 = 64;

// Ask Amit 
pub const RX_FIFO_SIZE: u16 = (4 * MAX_CONTROL_ENDPOINTS + 6) +
                              (2 * (MAX_PACKET_SIZE / 4 + 1)) +
                              (2 * MAX_NORMAL_ENDPOINTS) + 1;
pub const TX_FIFO_SIZE: u16 = 2 * MAX_PACKET_SIZE / 4;

#[derive(PartialEq)]
pub enum Interrupt {
    HostMode           = 1 <<  0,
    Mismatch           = 1 <<  1,
    OTG                = 1 <<  2,
    SOF                = 1 <<  3,
    RxFIFO             = 1 <<  4,
    GlobalInNak        = 1 <<  6,
    OutNak             = 1 <<  7,
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
pub enum OutInterruptMask {
    XferComplMsk =         1 <<  0,    
    EPDisbldMsg =          1 <<  1,
    AHBErrMsk =            1 <<  2,
    SetUPMsk =             1 <<  3,
    OUTTknEPdisMsk =       1 <<  4,
    StsPhseRcvdMsk =       1 <<  5,
    Back2BackSETupMsk =    1 <<  6,
    // Bit 7 reserved
    OutPkrErrMsk =         1 <<  8,
    BnaOutIntrMsk =        1 <<  9,
    // Bits 10-11 reserved
    BbleErrMsk =           1 << 12,
    NAKMsk =               1 << 13,
    NYETMsk =              1 << 14,
    // Bits 15-31 reserved
}

// OTG Databook, Table 5-57
#[allow(dead_code)]
pub enum InInterruptMask {
    XferComplMsk =         1 <<  0,    
    EPDisbldMsg =          1 <<  1,
    AHBErrMsk =            1 <<  2,
    TimeOUTMsk =           1 <<  3,
    INTknTXFEdmpMsk =      1 <<  4,
    INTknEPMisMsk =        1 <<  5,
    INTEPNakEffMsk =       1 <<  6,
    // Bit 7 reserved
    TxfifiUndrnMsk =       1 <<  8,
    BNAInIntrMsk =         1 <<  9,
    // Bits 10-12 reserved
    NAKMsk =               1 << 13,
    // Bits 14-31 reserved
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

