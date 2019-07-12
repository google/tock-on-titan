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

use kernel::ReturnCode;

use usb::constants::EP_BUFFER_SIZE_WORDS;
use usb::{Descriptor, StringDescriptor};

pub static mut STRINGS: [StringDescriptor; 7] = [
    StringDescriptor {
        b_length: 4,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0409], // English
    },
    StringDescriptor {
        b_length: 24,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0047, 0x006f, 0x006f, 0x0067, 0x006c, 0x0065, 0x0020, 0x0049, 0x006e, 0x0063, 0x002e], // Google Inc.
    },
    StringDescriptor {
        b_length: 14,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0070, 0x0072, 0x006f, 0x0074, 0x006f, 0x0032], // proto2
    },
    StringDescriptor {
        b_length: 54,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0070, 0x0072, 0x006F, 0x0074, 0x006F, 0x0032, 0x005F, 0x0076, 0x0031, 0x002E, 0x0031, 0x002E, 0x0038, 0x0037, 0x0031, 0x0033, 0x002D, 0x0030, 0x0031, 0x0033, 0x0032, 0x0031, 0x0037, 0x0064, 0x0039, 0x0031], // proto2-...
    },
    // Why does this need 3 l (0x6C)? Linux seems to be truncating last one.
    // Verified GetDescriptor for the String is returning complete information.
    // -pal
    StringDescriptor {
        b_length: 12,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0053, 0x0068, 0x0065, 0x006C, 0x006C, 0x006C], // Shell
    },
    StringDescriptor {
        b_length: 8,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0042, 0x004C, 0x0041, 0x0048],  // BLAH
    },
    StringDescriptor {
        b_length: 20,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0048, 0x006F, 0x0074, 0x0065, 0x006C, 0x0020, 0x0055, 0x0032, 0x0046], // Hotel U2F
    },
];


/// Trait a USB peripheral stack must implement to support the U2F syscall
/// capsule.
pub trait UsbHidU2f<'a> {
    fn power_down(&self) -> ReturnCode;
    fn power_up(&self) -> ReturnCode;

    fn set_u2f_client(&self, client: &'a UsbHidU2fClient<'a>);

    /// Reset the device and endpoints
    fn setup_u2f_descriptors(&self);

    /// For a reconnect: disconnect, wait, then connect
    fn force_reconnect(&self) -> ReturnCode;

    /// Enable reception of next frame; call after `get_slice` or `get_frame`.
    fn enable_rx(&self) -> ReturnCode;

    /// Sends the U2F report descriptor over the control channel (EP0)
    fn iface_respond(&self) -> ReturnCode;

    /// Blindly copies a frame out of the RXFIFO: run in response to `frame_received`.
    fn get_frame(&self, frame: &mut [u32; EP_BUFFER_SIZE_WORDS]);

    /// Blindly copies a frame out of the RXFIFO: run in response to `frame_received`.
    fn get_slice(&self, frame: &mut [u8]) -> ReturnCode;

    /// Returns whether the TXFIFO is available for sending.
    fn transmit_ready(&self) -> bool;

    /// Transmits a frame, fails if TXFIFO is not ready. Simple word copy (requires no byte
    /// reordering), use this when possible.
    fn put_frame(&self, frame: &[u32; EP_BUFFER_SIZE_WORDS]) -> ReturnCode;

    /// Transmits a frame, fails if TXFIFO is not ready. Requires byte-by-byte copy, use
    /// only when caller buffer couldn't be aligned or presized. Included to prevent
    /// double-copy from userspace buffers.
    fn put_slice(&self, frame: &[u8]) -> ReturnCode;
}

/// Client for the UsbHidU2f trait.
pub trait UsbHidU2fClient<'a> {
    fn reconnected(&self);
    fn frame_received(&self);
    fn frame_transmitted(&self);
}
