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

use core::ops::Deref;
use super::serialize::Serialize;
use usb::constants::Descriptor;
use usb::constants::MAX_PACKET_SIZE;
use usb::constants::U2F_REPORT_SIZE;

/// A StaticRef is a pointer to statically allocated mutable data such
/// as memory mapped I/O registers.
///
/// It is a simple wrapper around a raw pointer that encapsulates an
/// unsafe dereference in a safe manner. It serves the role of
/// creating a `&'static T` given a raw address and acts similarly to
/// `extern` definitions, except `StaticRef` is subject to module and
/// crate bounderies, while `extern` definitions can be imported
/// anywhere.
///
/// TODO(alevy): move into `common` crate or replace with other mechanism.
pub struct StaticRef<T> {
    ptr: *const T,
}

impl<T> StaticRef<T> {
    /// Create a new `StaticRef` from a raw pointer
    ///
    /// ## Safety
    ///
    /// Callers must pass in a reference to statically allocated memory which
    /// does not overlap with other values.
    pub const unsafe fn new(ptr: *const T) -> StaticRef<T> {
        StaticRef { ptr: ptr }
    }
}

impl<T> Deref for StaticRef<T> {
    type Target = T;
    fn deref(&self) -> &'static T {
        unsafe { &*self.ptr }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct DeviceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub bcd_usb: u16,
    pub b_device_class: u8,
    pub b_device_sub_class: u8,
    pub b_device_protocol: u8,
    pub b_max_packet_size0: u8,
    pub id_vendor: u16,
    pub id_product: u16,
    pub bcd_device: u16,
    pub i_manufacturer: u8,
    pub i_product: u8,
    pub i_serial_number: u8,
    pub b_num_configurations: u8,
}

impl DeviceDescriptor {
}

unsafe impl Serialize for DeviceDescriptor {}

#[derive(Debug)]
#[repr(C)]
pub struct ConfigurationDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub w_total_length: u16,
    pub b_num_interfaces: u8,
    pub b_configuration_value: u8,
    pub i_configuration: u8,
    pub bm_attributes: u8,
    pub b_max_power: u8,
}

const CONFIGURATION_DESCRIPTOR_LENGTH: u8 = 9;
impl ConfigurationDescriptor {
    /// Creates a configuration with `num_interfaces` and whose string
    /// descriptor is `i_configuration`. The value `b_max_power` sets
    /// the maximum power of the device in 2mA increments.  The
    /// configuration has `bm_attributes` set to bus powered (not
    /// remote wakeup).
    pub fn new(num_interfaces: u8,
               i_configuration: u8,
               b_max_power: u8) -> ConfigurationDescriptor {
        ConfigurationDescriptor {
            b_length: CONFIGURATION_DESCRIPTOR_LENGTH,
            b_descriptor_type: Descriptor::Configuration as u8,
            w_total_length: CONFIGURATION_DESCRIPTOR_LENGTH as u16,
            b_num_interfaces: num_interfaces,
            b_configuration_value: 1,
            i_configuration: i_configuration,
            bm_attributes: 0b10000000,
            b_max_power: b_max_power,
        }
    }

    /// Take the configuration and write it out as bytes into
    /// the u32 buffer, returning the number of bytes written.
    pub fn into_u32_buf(&self, buf: &mut [u32; 64]) -> usize {
        buf[0] = (self.b_length as u32)          <<  0 |
                 (self.b_descriptor_type as u32) <<  8 |
                 (self.w_total_length as u32)    << 16;
        buf[1] = (self.b_num_interfaces as u32)      <<  0 |
                 (self.b_configuration_value as u32) <<  8 |
                 (self.i_configuration as u32)       << 16 |
                 (self.bm_attributes as u32)         << 24;
        buf[2] = (self.b_max_power as u32) << 0;
        CONFIGURATION_DESCRIPTOR_LENGTH as usize
    }

    /// Take the configuration and write it out as a bytes into
    /// the u8 buffer, returning the number of bytes written
    pub fn into_u8_buf(&self, buf: &mut [u8]) -> usize {
        buf[0] = self.b_length as u8;
        buf[1] = self.b_descriptor_type as u8;
        buf[2] = self.w_total_length as u8;
        buf[3] = (self.w_total_length >> 8) as u8;
        buf[4] = self.b_num_interfaces as u8;
        buf[5] = self.b_configuration_value as u8;
        buf[6] = self.i_configuration as u8;
        buf[7] = self.bm_attributes as u8;
        buf[8] = self.b_max_power as u8;
        CONFIGURATION_DESCRIPTOR_LENGTH as usize
    }

    pub fn get_total_length(&self) -> u16 {
        self.w_total_length
    }

    pub fn set_total_length(&mut self, len: u16) {
        self.w_total_length = len;
    }

    pub fn length(&self) -> usize {
        CONFIGURATION_DESCRIPTOR_LENGTH as usize
    }
}


#[derive(Debug)]
pub struct StringDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub b_string: &'static [u16],
}

impl StringDescriptor {
    pub fn new(str: &'static [u16]) -> StringDescriptor {
        StringDescriptor {
            b_length: (str.len() * 2 + 2) as u8,
            b_descriptor_type: Descriptor::String as u8,
            b_string: str,
        }
    }

    pub fn into_u32_buf(&self, buf: &mut [u32; 64]) -> usize {
        let count = self.b_string.len();
        if count == 0 {
            buf[0] = (self.b_length as u32)          << 0 |
                     (self.b_descriptor_type as u32) << 8;
            2
        } else {
            buf[0] = (self.b_length as u32)          << 0 |
                     (self.b_descriptor_type as u32) << 8 |
                     (self.b_string[0] as u32)       << 16;
            for i in 1..count {
                // The first 16 bits of the message are the
                // length and type. The next 16 bits of the message are the first
                // wide character of the string (index 0). So this means that bits
                // 16..31 of buf[0] are b_string[0], bits 0..15 of buf[1] are string[1],
                // bits 16..31 of buf[1] are string[2].
                if i % 2 == 1 {
                    buf[(i / 2) + 1] = self.b_string[i] as u32;
                } else {
                    buf[i / 2] = buf[i / 2] | (self.b_string[i] as u32) << 16;
                }
            }
            2 + 2 * count
        }
    }

    pub fn length(&self) -> usize {
        self.b_length as usize
    }
}

unsafe impl Serialize for ConfigurationDescriptor {}

#[derive(Debug)]
pub struct InterfaceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub b_interface_number: u8,
    pub b_alternate_setting: u8,
    pub b_num_endpoints: u8,
    pub b_interface_class: u8,
    pub b_interface_sub_class: u8,
    pub b_interface_protocol: u8,
    pub i_interface: u8
}

impl InterfaceDescriptor {
    // This is the interface descriptor for a FIDO U2F device.
    // Taken from Section 3.1 of FIDO U2F HID protocol document.
    pub fn new(interface_string: u8, which: u8, class: u8, sub_class: u8, protocol: u8) -> InterfaceDescriptor {
        InterfaceDescriptor {
            b_length: 9,
            b_descriptor_type: 4,        // Interface descriptor
            b_interface_number: which,
            b_alternate_setting: 0,
            b_num_endpoints: 2,
            b_interface_class: class,
            b_interface_sub_class: sub_class,
            b_interface_protocol: protocol,
            i_interface: interface_string,
        }
    }

    /// Take the interface and write it out as bytes into
    /// the u32 buffer, returning the number of bytes written.
    pub fn into_u32_buf(&self, buf: &mut [u32; 64]) -> usize {
        buf[0] = (self.b_length as u32)              <<  0 |
                 (self.b_descriptor_type as u32)     <<  8 |
                 (self.b_interface_number as u32)    << 16 |
                 (self.b_alternate_setting as u32)   << 24;
        buf[1] = (self.b_num_endpoints as u32)       <<  0 |
                 (self.b_interface_class as u32)     <<  8 |
                 (self.b_interface_sub_class as u32) << 16 |
                 (self.b_interface_protocol as u32)  << 24;
        buf[2] = (self.i_interface as u32)           <<  0;
        9
    }

    /// Take the interface and write it out as bytes into the u8
    /// buffer, returning the number of bytes written.
    pub fn into_u8_buf(&self, buf: &mut [u8]) -> usize {
        buf[0] = self.b_length;
        buf[1] = self.b_descriptor_type;
        buf[2] = self.b_interface_number;
        buf[3] = self.b_alternate_setting;
        buf[4] = self.b_num_endpoints;
        buf[5] = self.b_interface_class;
        buf[6] = self.b_interface_sub_class;
        buf[7] = self.b_interface_protocol;
        buf[8] = self.i_interface;
        9
    }

    pub fn length(&self) -> usize {
        9
    }
}

#[repr(u8)]
#[derive(Debug)]
pub enum EndpointTransferType {
    Control     = 0b00,
    Isochronous = 0b01,
    Bulk        = 0b10,
    Interrupt   = 0b11,
}

#[repr(u8)]
#[derive(Debug)]
pub enum EndpointSynchronizationType {
    None         = 0b00,
    Asynchronous = 0b01,
    Adaptive     = 0b10,
    Synchronous  = 0b11
}

#[repr(u8)]
#[derive(Debug)]
pub enum EndpointUsageType {
    Data             = 0b00,
    Feedback         = 0b01,
    ExplicitFeedback = 0b10,
    Reserved         = 0b11,
}


#[derive(Debug)]
pub struct EndpointAttributes {
    pub transfer: EndpointTransferType,
    pub synchronization: EndpointSynchronizationType,
    pub usage: EndpointUsageType,
}

impl Into<u8> for EndpointAttributes {
    fn into(self) -> u8 {
        match self.transfer {
            EndpointTransferType::Isochronous => self.transfer as u8,
            _ => {
                self.transfer as u8 |
                (self.synchronization as u8) << 2 |
                (self.usage as u8) << 4
            }
        }
    }
}

impl From<u8> for EndpointAttributes {
    fn from(val: u8) -> EndpointAttributes {
        EndpointAttributes {
            transfer: match val & 0b11 {
                0b00 => EndpointTransferType::Control,
                0b01 => EndpointTransferType::Isochronous,
                0b10 => EndpointTransferType::Bulk,
                0b11 => EndpointTransferType::Interrupt,
                _ => EndpointTransferType:: Control,
            },
            synchronization: match (val >> 2) & 0b11 {
                0b00 => EndpointSynchronizationType::None,
                0b01 => EndpointSynchronizationType::Asynchronous,
                0b10 => EndpointSynchronizationType::Adaptive,
                0b11 => EndpointSynchronizationType::Synchronous,
                _ => EndpointSynchronizationType::None,
            },
            usage: match (val >> 4) & 0b11 {
                0b00 => EndpointUsageType::Data,
                0b01 => EndpointUsageType::Feedback,
                0b10 => EndpointUsageType::ExplicitFeedback,
                _ => EndpointUsageType::Reserved
            }
        }
    }
}

#[derive(Debug)]
pub struct EndpointDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub b_endpoint_address: u8,
    pub bm_attributes: u8,
    pub w_max_packet_size: u16,
    pub b_interval: u8
}

impl EndpointDescriptor {
    pub fn new(address: u8, attributes: EndpointAttributes, interval: u8) -> EndpointDescriptor {
        EndpointDescriptor {
            b_length: 7,
            b_descriptor_type: Descriptor::Interface as u8,
            b_endpoint_address: address,
            bm_attributes: attributes.into(),
            w_max_packet_size: MAX_PACKET_SIZE,
            b_interval: interval,
        }
    }

    pub fn into_u8_buf(&self, buf: &mut [u8]) -> usize {
        buf[0] = 7;
        buf[1] = Descriptor::Endpoint as u8;
        buf[2] = self.b_endpoint_address;
        buf[3] = self.bm_attributes;
        buf[4] = self.w_max_packet_size as u8;
        buf[5] = (self.w_max_packet_size >> 8) as u8;
        buf[6] = self.b_interval;
        7
    }

    pub fn length(&self) -> usize {
        7
    }
}

// This is a hardcoded HID device descriptor: a fully general one
// is out of scope right now. -plevis 9/27/18
#[derive(Debug)]
pub struct HidDeviceDescriptor {
    b_length: u8,
    b_descriptor_type: u8,
    w_release: u16,
    b_country: u8,
    b_descriptors: u8,
    b_sub_descriptor_type: u8,
    w_sub_descriptor_length: u16,
}

impl HidDeviceDescriptor {
    pub fn new() -> HidDeviceDescriptor {
        HidDeviceDescriptor {
            b_length: 9,
            b_descriptor_type: Descriptor::HidDevice as u8,
            w_release: 0x0100,
            b_country: 0,
            b_descriptors: 1,
            b_sub_descriptor_type: 34, // Report
            w_sub_descriptor_length: 34
        }
    }

    pub fn into_u8_buf(&self, buf: &mut [u8]) -> usize {
        buf[0] = self.b_length;
        buf[1] = self.b_descriptor_type;
        buf[2] = self.w_release as u8;
        buf[3] = (self.w_release >> 8) as u8;
        buf[4] = self.b_country;
        buf[5] = self.b_descriptors;
        buf[6] = self.b_sub_descriptor_type;
        buf[7] = self.w_sub_descriptor_length as u8;
        buf[8] = (self.w_sub_descriptor_length >> 8) as u8;
        9
    }

    pub fn length(&self) -> usize {
        9
    }
}


#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
#[repr(u8)]
pub enum SetupRequestType {
    GetStatus = 0,
    ClearFeature = 1,
    Reserved = 2,
    SetFeature = 3,

    SetAddress = 5,
    GetDescriptor = 6,
    SetDescriptor = 7,
    GetConfiguration = 8,
    SetConfiguration = 9,
    GetInterface = 10,
    SetInterface = 11,
    SynchFrame = 12,
    Undefined = 15,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
#[repr(u8)]
pub enum SetupClassRequestType {
    Undefined = 0,
    SetIdle = 10,
}


#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SetupDirection {
    HostToDevice = 0,
    DeviceToHost = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SetupRequestClass {
    Standard = 0,
    Class    = 1,
    Vendor   = 2,
    Reserved = 3,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SetupRecipient {
    Device    = 0,
    Interface = 1,
    Endpoint  = 2,
    Other     = 3,
    Reserved  = 4,
}

#[derive(Debug)]
pub struct SetupRequest {
    pub bm_request_type: u8,
    pub b_request: u8,
    pub w_value: u16,
    pub w_index: u16,
    pub w_length: u16,
}

impl SetupRequest {

    pub fn new(buf: &[u32; 16]) -> SetupRequest {
        SetupRequest {
            bm_request_type: (buf[0] & 0xff) as u8,
            b_request:      ((buf[0] & 0x0000ff00) >> 8) as u8,
            w_value:        ((buf[0] & 0xffff0000) >> 16) as u16,
            w_index:         (buf[1] & 0x0000ffff) as u16,
            w_length:       ((buf[1] & 0xffff0000) >> 16) as u16,
        }
    }

#[allow(dead_code)]
    pub fn parse(buf: &[u32; 16], req: &mut SetupRequest) {
        req.bm_request_type = (buf[0] & 0xff) as u8;
        req.b_request =      ((buf[0] & 0x0000ff00) >> 8) as u8;
        req.w_value =        ((buf[0] & 0xffff0000) >> 16) as u16;
        req.w_index =         (buf[1] & 0x0000ffff) as u16;
        req.w_length =       ((buf[1] & 0xffff0000) >> 16) as u16
    }

    // 0 is Host-to-Device, 1 is Device-to-Host
    pub fn data_direction(&self) -> SetupDirection {
        let val = (self.bm_request_type & 0x80) >> 7;
        match val {
            0 => SetupDirection::HostToDevice,
            _ => SetupDirection::DeviceToHost
        }
    }

    // 0 is Standard, 1 is Class, 2 is Vendor, 3 is Reserved
    pub fn req_type(&self) -> SetupRequestClass {
        let val = (self.bm_request_type & 0x60) >> 5;
        match val {
            0 => SetupRequestClass::Standard,
            1 => SetupRequestClass::Class,
            2 => SetupRequestClass::Vendor,
            _ => SetupRequestClass::Reserved,
        }
    }

    // 0 is Device, 1 is Interface, 2 is Endpoint, 3 is Other
    // 4..31 are Reserved
    pub fn recipient(&self) -> SetupRecipient {
        let val = self.bm_request_type & 0x1f;
        match val {
            0 => SetupRecipient::Device,
            1 => SetupRecipient::Interface,
            2 => SetupRecipient::Endpoint,
            3 => SetupRecipient::Other,
            _ => SetupRecipient::Reserved,
        }
    }

    pub fn class_request(&self) -> SetupClassRequestType {
        match self.b_request {
            10 => SetupClassRequestType::SetIdle,
            _  => SetupClassRequestType::Undefined,
        }
    }

    pub fn request(&self) -> SetupRequestType {
        match self.b_request {
            0 => SetupRequestType::GetStatus,
            1 => SetupRequestType::ClearFeature,
            2 => SetupRequestType::Reserved,
            3 => SetupRequestType::SetFeature,
            4 => SetupRequestType::Reserved,
            5 => SetupRequestType::SetAddress,
            6 => SetupRequestType::GetDescriptor,
            7 => SetupRequestType::SetDescriptor,
            8 => SetupRequestType::GetConfiguration,
            9 => SetupRequestType::SetConfiguration,
            10 => SetupRequestType::GetInterface,
            11 => SetupRequestType::SetInterface,
            12 => SetupRequestType::SynchFrame,
             _ => SetupRequestType::Undefined
        }
    }

    pub fn value(&self) -> u16 {
        self.w_value
    }

    pub fn index(&self) -> u16 {
        self.w_index
    }

    pub fn length(&self) -> u16 {
        self.w_length
    }
}

pub struct U2fHidCommandFrame {
    pub channel_id: u32,
    pub frame_type: u8,
    pub command: u8,
    pub bcount_high: u8,
    pub bcount_low: u8,
    pub data: [u8; U2F_REPORT_SIZE as usize - 8],
}

impl U2fHidCommandFrame {
    pub fn into_u32_buf(&self, buf: &mut [u32; 16]) {
        buf[0] = self.channel_id;
        buf[1] = (self.bcount_low as u32) << 24 |
                 (self.bcount_high as u32) << 16 |
                 (self.command as u32) << 8 |
                 (self.frame_type as u32) << 0;
        buf[2] = (self.data[0] as u32) << 24 |
                 (self.data[1] as u32) << 16 |
                 (self.data[2] as u32) << 8 |
                 (self.data[3] as u32) << 0;

    }
}

pub struct U2fHidSequenceFrame {
    channel_id: u32,
    frame_type: u8,
    sequence_num: u8,
    data: [u8; U2F_REPORT_SIZE as usize - 6],
}
