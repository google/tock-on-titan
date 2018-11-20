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

use core::ops::{BitAnd, BitOr};
use kernel::common::cells::VolatileCell;

#[repr(C)]
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
    // 0x38
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
    // 0x828
    pub device_vbus_discharge_time: VolatileCell<u32>,
    pub device_vbus_pulsing_time: VolatileCell<u32>,
    pub device_threshold_control: VolatileCell<u32>,
    pub device_in_ep_fifo_empty_interrupt_mask: VolatileCell<u32>,

    _reserved_5: [u32; 50],
    // 0x900
    pub in_endpoints: [InEndpoint; 16],
    pub out_endpoints: [OutEndpoint; 16],
    // 0xd00
    _reserved6: [u32; 64],
    // 0xe00
    pub power_clock_gating_control: VolatileCell<u32>,
}

#[repr(C)]
pub struct InEndpoint {
    pub control: VolatileCell<EpCtl>,
    _reserved0: u32,
    pub interrupt: VolatileCell<u32>,
    _reserved1: u32,
    // We use scatter-gather mode so transfer-size isn't used
    _transfer_size: VolatileCell<u32>,
    pub dma_address: VolatileCell<&'static DMADescriptor>,
    pub tx_fifo_status: VolatileCell<u32>,
    pub buffer_address: VolatileCell<u32>,
}

#[repr(C)]
pub struct OutEndpoint {
    pub control: VolatileCell<EpCtl>,
    _reserved0: u32,
    pub interrupt: VolatileCell<u32>,
    _reserved1: u32,
    _transfer_size: VolatileCell<u32>,
    pub dma_address: VolatileCell<&'static DMADescriptor>,
    _reserved2: u32,
    pub buffer_address: VolatileCell<u32>,
}

/// In/Out Endpoint Control flags
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EpCtl(pub u32);

impl EpCtl {
    /// Enable the endpoint
    pub const ENABLE: EpCtl    = EpCtl(1 << 31);
    /// Clear endpoint NAK
    pub const CNAK: EpCtl      = EpCtl(1 << 26);
    /// Set the 4-bit TxFIFO number to be 1
    pub const TXFNUM1: EpCtl   = EpCtl(1 << 22);
    /// Stall endpoint
    pub const STALL: EpCtl     = EpCtl(1 << 21);
    /// Make an endpoint of type Interrupt
    pub const INTERRUPT: EpCtl = EpCtl(3 << 18);
    /// Denotes whether endpoint is active
    pub const USBACTEP: EpCtl  = EpCtl(1 << 15);

}

impl BitOr for EpCtl {
    type Output = Self;
    fn bitor(self, rhs: EpCtl) -> EpCtl {
        EpCtl(self.0 | rhs.0)
    }
}

impl BitAnd for EpCtl {
    type Output = Self;
    fn bitand(self, rhs: EpCtl) -> EpCtl {
        EpCtl(self.0 & rhs.0)
    }
}

#[repr(C)]
#[repr(align(4))]
#[derive(Clone, Copy, Debug)]
pub struct DMADescriptor {
    pub flags: DescFlag,
    pub addr: usize,
}

/// Status quadlet for a DMA descriptor
///
/// The status quadlet is a 32-bit flag register in the DMA descriptor that
/// reflects the status of the descriptor. It can mark whether the Host/DMA is
/// ready to transmit/receive this descriptor and describes how large the buffer
/// is.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DescFlag(pub u32);

impl BitOr for DescFlag {
    type Output = Self;
    fn bitor(self, rhs: DescFlag) -> DescFlag {
        DescFlag(self.0 | rhs.0)
    }
}

impl BitAnd for DescFlag {
    type Output = Self;
    fn bitand(self, rhs: DescFlag) -> DescFlag {
        DescFlag(self.0 & rhs.0)
    }
}

impl DescFlag {
    /// This descriptor is the last in a transmission
    pub const LAST: DescFlag = DescFlag(1 << 27);
    /// This descriptor describes a short transfer
    pub const SHORT: DescFlag = DescFlag(1 << 26);
    /// Generate an interrupt on completion
    pub const IOC: DescFlag = DescFlag(1 << 25);
    /// Indicates that a setup packet has been received
    pub const SETUP_READY: DescFlag = DescFlag(1 << 24);

    /// Host Ready status
    pub const HOST_READY: DescFlag = DescFlag(0b00 << 30);
    /// DMA Busy status
    pub const DMA_BUSY: DescFlag = DescFlag(0b01 << 30);
    /// DMA Ready status
    pub const DMA_DONE: DescFlag = DescFlag(0b10 << 30);
    /// Host Busy status
    pub const HOST_BUSY: DescFlag = DescFlag(0b11 << 30);

    /// Set the number of bytes to transmit
    pub const fn bytes(self, bytes: u16) -> DescFlag {
        DescFlag(self.0 | bytes as u32)
    }

    pub const fn to_u32(self) -> u32 {
        self.0
    }
}
