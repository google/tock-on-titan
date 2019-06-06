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
pub enum Gpio {
    PhyA       = 0b100 << 4,
    PhyB       = 0b101 << 4,
    WriteMode  = 1 << 15,
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
