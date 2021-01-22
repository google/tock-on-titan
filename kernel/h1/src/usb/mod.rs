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

pub mod constants;
pub mod driver;
mod registers;
mod serialize;
pub mod types;
pub mod u2f;

pub use self::constants::Descriptor;
pub use self::types::StringDescriptor;

use core::cell::Cell;
use cortexm3::support;
use kernel::ReturnCode;
use kernel::common::cells::{OptionalCell, TakeCell};
use kernel::common::registers::{LocalRegisterCopy};
use crate::pmu::{Clock, PeripheralClock, PeripheralClock1};

use self::constants::*;
use self::registers::{AhbConfig, AllEndpointInterrupt, DescFlag,
                      DeviceConfig, DeviceControl, DMADescriptor,
                      EndpointControl, Gpio, InEndpointInterruptMask,
                      Interrupt, OutEndpointInterruptMask, Registers,
                      Reset, UsbConfiguration};
use self::types::{ConfigurationDescriptor, DeviceDescriptor,
                  EndpointAttributes, EndpointDescriptor,
                  EndpointSynchronizationType, EndpointTransferType,
                  EndpointUsageType, HidDeviceDescriptor,
                  InterfaceDescriptor, SetupDirection, SetupRecipient,
                  SetupRequest, SetupRequestClass, SetupRequestType,
                  StaticRef};
use self::u2f::{UsbHidU2f, UsbHidU2fClient};

// Simple macros for USB debugging output: default definitions do nothing,
// but you can uncomment print defintions to get detailed output on the
// messages sent and received.

macro_rules! control_debug { // Debug messages for enumeration/EP0 control
//    () => ({print!();});
//    ($fmt:expr) => ({print!($fmt);});
//    ($fmt:expr, $($arg:tt)+) => ({print!($fmt, $($arg)+);});
    () => ({});
    ($fmt:expr) => ({});
    ($fmt:expr, $($arg:tt)+) => ({});
}

macro_rules! data_debug { // Debug messages for data/EP1
//    () => ({print!();});
//    ($fmt:expr) => ({print!($fmt);});
//    ($fmt:expr, $($arg:tt)+) => ({print!($fmt, $($arg)+);});
    () => ({});
    ($fmt:expr) => ({});
    ($fmt:expr, $($arg:tt)+) => ({});
}

macro_rules! int_debug { // Debug messages for interrupt handling
//    () => ({print!();});
//    ($fmt:expr) => ({print!($fmt);});
//    ($fmt:expr, $($arg:tt)+) => ({print!($fmt, $($arg)+);});
    () => ({});
    ($fmt:expr) => ({});
    ($fmt:expr, $($arg:tt)+) => ({});
}

/// USBState encodes the current state of the USB driver's state
/// machine. It can be in three states: waiting for a message from
/// the host, sending data in reply to a query from the host, or sending
/// a status response (no data) in reply to a command from the host.
#[derive(Clone, Copy, PartialEq, Eq)]
enum USBState {
    WaitingForSetupPacket,   // Waiting for message from host
    DataStageIn,             // Sending data to host
    NoDataStage,             // Sending status (not data) to host,
    // e.g. in response to set command
}

// Constants for how many buffers to use for EP0.
const EP0_IN_BUFFER_COUNT:  usize = 4;
const EP0_OUT_BUFFER_COUNT: usize = 2;

/// Driver for the Synopsys DesignWare Cores USB 2.0 Hi-Speed
/// On-The-Go (OTG) controller.
///
/// Page/figure references are for the Synopsys DesignWare Cores USB
/// 2.0 Hi-Speed On-The-Go (OTG) Programmer's Guide.
///
/// The driver can enumerate (appear as a device to a host OS) and
/// exchange data on EP1. The driver operates as a device in
/// Scatter-Gather DMA mode (Figure 1-1) and performs the initial
/// handshakes with the host on endpoint 0. An uninitialized drive
/// appears as a counterfeit flash device (vendor id: 0011, product
/// id: 7788); a call to init() should initialize the correct vendor
/// ID and product ID.
///
/// Scatter-gather mode operates using lists of descriptors. Each
/// descriptor points to a 64 byte memory buffer. A transfer larger
/// than 64 bytes uses multiple descriptors in sequence. An IN
/// descriptor is for sending to the host (the data goes IN to the
/// host), while an OUT descriptor is for receiving from the host (the
/// data goes OUT of the host).
///
/// For endpoint 0, the driver configures 2 OUT descriptors and 4 IN
/// descriptors. Four IN descriptors allows responses up to 256 bytes
/// (64 * 4), which is important for sending the device configuration
/// descriptor as one big blob.  The driver never expects to receive
/// OUT packets larger than 64 bytes (the maximum each descriptor can
/// handle). It uses two OUT descriptors so it can receive a packet
/// while processing the previous one.
///
/// The USB stack currently assumes the presence of 7
/// StringDescriptors, which are provided by the boot sequence. The
/// meaning of each StringDescriptor is defined by its index, in
/// usb::constants.

pub struct USB<'a> {
    registers: StaticRef<Registers>,
    core_clock: Clock,
    timer_clock: Clock,
    state: Cell<USBState>,

    // Descriptor and buffers should exist after a call to init.

    // EP0 is used for control messages (enumeration, etc.): they
    // are handled by the kernel (this module).
    // EP0 out (data out from host to device) is stored as two
    // separate buffers for double-buffering. EP1 in (data into
    // host from device) is one large buffer so this firmware
    // can easily copy larger objects into it. The EP0 in descriptors
    // can index into this larger buffer.
    ep0_out_descriptors: TakeCell<'static, [DMADescriptor; EP0_OUT_BUFFER_COUNT]>,
    ep0_out_buffers: Cell<Option<&'static [[u32; 16]; EP0_OUT_BUFFER_COUNT]>>,
    ep0_in_descriptors: TakeCell<'static, [DMADescriptor; EP0_IN_BUFFER_COUNT]>,
    ep0_in_buffers: TakeCell<'static, [u32; 16 * EP0_IN_BUFFER_COUNT]>,

    // Track the index of which ep0_out descriptor is currently set
    // for reception and which descriptor received the most
    // recent packet.
    next_ep0_out_idx: Cell<usize>,
    last_ep0_out_idx: Cell<usize>,

    // EP1 is used for application messages: userspace applications
    // can communicate using them through system calls. EP1 in and out
    // are both 64-byte buffers.
    ep1_out_descriptor: TakeCell<'static, DMADescriptor>,
    ep1_out_buffer: Cell<Option<&'static [u32; EP_BUFFER_SIZE_WORDS]>>,
    ep1_in_descriptor: TakeCell<'static, DMADescriptor>,
    ep1_in_buffer: TakeCell<'static,[u32; EP_BUFFER_SIZE_WORDS]>,


    // Numeric configurations set by instantation. These values are
    // filled into USB Descriptors as part of enumeration.
    device_class: Cell<u8>,
    vendor_id: Cell<u16>,
    product_id: Cell<u16>,

    // `configuration_descriptor` stores the bytes of the full USB
    // ConfigurationDescriptor. `configuration_total_length` is the
    // length. The function `generate_full_configuration_descriptor`
    // populates these values. The ConfigurationDescriptor is limited
    // to a single 64 byte buffer.
    configuration_descriptor: TakeCell<'static, [u8; EP_BUFFER_SIZE_BYTES]>,
    configuration_total_length: Cell<u16>,

    // Which USB configuration is currently being used.
    configuration_current_value: Cell<u8>,

    // The strings of the USB StringDescriptors (vendor name, device name,
    // etc.). Because different Descriptors index into this array, changing
    // the number of elements or their ordering requires changing other
    // aspects of code (e.g., `generate_full_configuration_descriptor`).
    strings: TakeCell<'static, [StringDescriptor]>,

    // Client to give callbacks to.
    u2f_client: OptionalCell<&'a dyn UsbHidU2fClient<'a>>,
}

// Hardware base address of the singleton USB controller
const BASE_ADDR: *const Registers = 0x40300000 as *const Registers;
pub static mut USB0: USB<'static> = unsafe { USB::new() };

impl<'a> USB<'a> {
    /// Creates a new value referencing the single USB driver.  After
    /// instantiation, init() needs to be called to initialize buffers
    /// and identifiers.
    ///
    /// ## Safety
    ///
    /// Callers must ensure this is only called once for every program
    /// execution. Creating multiple instances will result in conflicting
    /// handling of events and can lead to undefined behavior.
    const unsafe fn new() -> USB<'a> {
        USB {
            registers: StaticRef::new(BASE_ADDR),
            core_clock: Clock::new(PeripheralClock::Bank1(PeripheralClock1::Usb0)),
            timer_clock: Clock::new(PeripheralClock::Bank1(PeripheralClock1::Usb0TimerHs)),
            state: Cell::new(USBState::WaitingForSetupPacket),
            ep0_out_descriptors: TakeCell::empty(),
            ep0_out_buffers: Cell::new(None),
            ep0_in_descriptors: TakeCell::empty(),
            ep0_in_buffers: TakeCell::empty(),
            ep1_out_descriptor: TakeCell::empty(),
            ep1_out_buffer: Cell::new(None),
            ep1_in_descriptor: TakeCell::empty(),
            ep1_in_buffer: TakeCell::empty(),
            configuration_descriptor: TakeCell::empty(),
            next_ep0_out_idx: Cell::new(0),
            last_ep0_out_idx: Cell::new(0),
            device_class: Cell::new(0x00),
            vendor_id: Cell::new(0x0011),   // Dummy values for a bad USB device, should
            product_id: Cell::new(0x7788),  // be replaced in call to init()
            configuration_current_value: Cell::new(0),
            configuration_total_length: Cell::new(0),
            strings: TakeCell::empty(),
            u2f_client: OptionalCell::empty(),
        }
    }

    /// Initialize descriptors for endpoint 0 IN and OUT, resetting
    /// them to a clean state.
    fn init_ep0_descriptors(&self) {
        // Setup descriptor for OUT endpoint 0
        self.ep0_out_descriptors.map(|descs| {
            self.ep0_out_buffers.get().map(|bufs| {
                for (desc, buf) in descs.iter_mut().zip(bufs.iter()) {
                    desc.flags = DescFlag::HOST_BUSY;
                    desc.addr = buf.as_ptr() as usize;
                }
                self.next_ep0_out_idx.set(0);
                self.registers.out_endpoints[0].dma_address.set(&descs[0]);
            });
        });

        // Setup descriptor for IN endpoint 0
        self.ep0_in_buffers.map(|buf| {
            self.ep0_in_descriptors.map(|descs| {
                for (i, desc) in descs.iter_mut().enumerate() {
                    desc.flags = DescFlag::HOST_BUSY;
                    desc.addr = buf.as_ptr() as usize + i * 64;
                }
                self.registers.in_endpoints[0].dma_address.set(&descs[0]);
            });
        });
    }

    /// Reset the device in response to a USB RESET.
    fn usb_reset(&self) {
        control_debug!("USB: WaitingForSetupPacket in reset.\n");
        self.state.set(USBState::WaitingForSetupPacket);
        // Reset device address field (bits 10:4) of device config
        self.registers.device_config.modify(DeviceConfig::DeviceAddress.val(0));
        self.init_ep0_descriptors();
        self.expect_setup_packet();
    }

    fn ep1_tx_fifo_is_ready(&self) -> bool {
        self.ep1_in_descriptor.map_or(false, |desc| {
            desc.flags & DescFlag::STATUS_MASK == DescFlag::DMA_DONE ||
                desc.flags & DescFlag::STATUS_MASK == DescFlag::HOST_BUSY
        })
    }

    fn ep1_rx_fifo_is_ready(&self) -> bool {
        self.ep1_out_descriptor.map_or(false, |desc| {
            desc.flags & DescFlag::DMA_DONE == DescFlag::DMA_DONE
        })
    }

    fn ep1_enable_tx(&self) {
        self.ep1_in_descriptor.map(|desc| {
            desc.flags = (DescFlag::LAST |
                          DescFlag::HOST_READY |
                          DescFlag::IOC).bytes(U2F_REPORT_SIZE);
            self.registers.in_endpoints[1].control.modify(EndpointControl::Enable::SET +
                                                          EndpointControl::ClearNak::SET);
        });
    }

    fn ep1_enable_rx(&self) -> ReturnCode {
        self.ep1_out_descriptor.map_or(ReturnCode::FAIL, |desc| {
            desc.flags = (DescFlag::LAST |
                          DescFlag::HOST_READY |
                          DescFlag::IOC).bytes(U2F_REPORT_SIZE);
            self.registers.out_endpoints[1].control.modify(EndpointControl::Enable::SET +
                                                           EndpointControl::ClearNak::SET);
            data_debug!("Set EP1 receive flags.\n");
            ReturnCode::SUCCESS
        })
    }

    fn usb_reconnect(&self) {}

    /// Perform a soft reset on the USB core; timeout if the reset
    /// takes too long.
    fn soft_reset(&self) {
        // Reset and wait until flag is cleared or timeout
        self.registers.reset.write(Reset::PiuFsDedicatedControllerSoftReset::SET);
        let mut timeout = 10000;
        while self.registers.reset.is_set(Reset::PiuFsDedicatedControllerSoftReset) {
            if timeout == 0 {
                return;
            }
            timeout -= 1;
        }

        // Wait until Idle flag is set or timeout
        let mut timeout = 10000;
        while self.registers.reset.is_set(Reset::AhbMasterIdle) {
            if timeout == 0 {
                return;
            }
            timeout -= 1;
        }
    }

    /// The chip should call this interrupt bottom half from its
    /// `service_pending_interrupts` routine when an interrupt is
    /// received on the USB nvic line.
    ///
    /// Directly handles events related to device initialization, connection and
    /// disconnection, as well as control transfers on endpoint 0. Other events
    /// are passed to clients delegated for particular endpoints or interfaces.
    pub fn handle_interrupt(&self) {
        // Save current interrupt status snapshot to correctly clear at end
        let status = self.registers.interrupt_status.extract();
        let mask = self.registers.interrupt_mask.extract();

        print_usb_interrupt_status(status);

        if status.is_set(Interrupt::EnumerationDone) {
            // MPS default set to 0 == 64 bytes
            // "Application must read the DSTS register to obtain the
            //  enumerated speed."
        }

        if status.is_set(Interrupt::EarlySuspend) ||
            status.is_set(Interrupt::Suspend) {
                // Currently do not support suspend
            }

        if mask.is_set(Interrupt::StartOfFrame) &&
            status.is_set(Interrupt::StartOfFrame) { // Clear SOF
                self.registers.interrupt_mask.modify(Interrupt::StartOfFrame::CLEAR);
            }

        if status.is_set(Interrupt::Reset) ||
            status.is_set(Interrupt::ResetDetected) {
                self.usb_reset();
            }

        if status.is_set(Interrupt::InEndpoints) ||
            status.is_set(Interrupt::OutEndpoints) { // Interrupt pending
                let pending_interrupts = self.registers.device_all_ep_interrupt.extract();
                let inter_ep0_out = pending_interrupts.is_set(AllEndpointInterrupt::OUT0);
                let inter_ep0_in = pending_interrupts.is_set(AllEndpointInterrupt::IN0);
                let inter_ep1_out = pending_interrupts.is_set(AllEndpointInterrupt::OUT1);
                let inter_ep1_in = pending_interrupts.is_set(AllEndpointInterrupt::IN1);
                int_debug!(" - handling endpoint interrupts {:032b}\n", pending_interrupts.get());
                int_debug!(" -      all endpoint mask       {:032b}\n", self.registers.device_all_ep_interrupt_mask.get());
                int_debug!(" -     out1 endpoint ints       {:032b}\n", self.registers.out_endpoints[1].interrupt.get());
                int_debug!(" -      in1 endpoint ints       {:032b}\n", self.registers.in_endpoints[1].interrupt.get());
                int_debug!("                   debug reg    {:032b}\n", self.registers._grxstsr.get());
                if inter_ep0_out || inter_ep0_in {
                    int_debug!("   - ep0out: {} ep0in: {}\n", inter_ep0_out, inter_ep0_in);
                    self.handle_endpoint0_events(inter_ep0_out, inter_ep0_in);
                } else if inter_ep1_out || inter_ep1_in {
                    int_debug!("   - ep1out: {} ep1in: {}\n", inter_ep1_out, inter_ep1_in);
                    self.handle_endpoint1_events(inter_ep1_out, inter_ep1_in);
                }
            }

        // Clear Global OUT NAK
        if status.is_set(Interrupt::GlobalOutNak) {
            self.registers.device_control.modify(DeviceControl::ClearGlobalOutNak::SET);
        }

        // Clear Global Non-periodic IN NAK
        if status.is_set(Interrupt::GlobalInNak) {
            self.registers.device_control.modify(DeviceControl::ClearGlobalNonPeriodicInNak::SET);
        }

        self.registers.interrupt_status.set(status.get());
    }

    /// Set up endpoint 0 OUT descriptors to receive a setup packet
    /// from the host, whose reception will trigger an interrupt.
    /// Preparing for a SETUP packet disables IN interrupts (device
    /// should not be sending anything) and enables OUT interrupts
    /// (for reception from host).
    //
    // A SETUP packet is less than 64 bytes, so only one OUT
    // descriptor is needed. This function sets the max size of the
    // packet to 64 bytes the Last and Interrupt-on-completion bits
    // and max size to 64 bytes.
    fn expect_setup_packet(&self) {
        control_debug!("USB: WaitingForSetupPacket in expect_setup_packet.\n");
        self.state.set(USBState::WaitingForSetupPacket);
        self.ep0_out_descriptors.map(|descs| {
            descs[self.next_ep0_out_idx.get()].flags =
                (DescFlag::HOST_READY | DescFlag::LAST | DescFlag::IOC).bytes(64);
        });

        // Enable OUT and disable IN interrupts
        self.registers.device_all_ep_interrupt_mask.modify(AllEndpointInterrupt::OUT0::SET + AllEndpointInterrupt::IN0::CLEAR);
        //self.registers.device_all_ep_interrupt_mask.set(interrupts);

        // Clearing the NAK bit tells host that device is ready to receive.
        self.registers.out_endpoints[0].control.write(EndpointControl::Enable::SET +
                                                      EndpointControl::ClearNak::SET);
    }

    /// Handles events for endpoint 1 (data to/from USB client). Clear
    /// pending interrupts and issue callbcks to client.
    fn handle_endpoint1_events(&self, out_interrupt: bool, in_interrupt: bool) {
        data_debug!("Handling endpoint 1 events: out {}, in {}\n", out_interrupt, in_interrupt);
        if in_interrupt {
            let ep_in = &self.registers.in_endpoints[1];
            let ep_in_interrupts = ep_in.interrupt.extract();
            data_debug!("In interrupts: {:#x}\n", ep_in_interrupts.get());
            print_in_endpoint_interrupt_status(ep_in_interrupts);
            ep_in.interrupt.set(ep_in_interrupts.get());
            if ep_in_interrupts.is_set(InEndpointInterruptMask::TransferCompleted) {
                data_debug!("U2F: frame_transmitted callback on ep1.\n");
                self.u2f_client.map(|client| client.frame_transmitted());
            }

        }
        if out_interrupt {
            let ep_out = &self.registers.out_endpoints[1];
            let ep_out_interrupts = ep_out.interrupt.extract();
            data_debug!("Out interrupts: {:#x}\n", ep_out_interrupts.get());
            ep_out.interrupt.set(ep_out_interrupts.get());
            if ep_out_interrupts.is_set(OutEndpointInterruptMask::TransferCompleted) {
                data_debug!("U2F: ep1 frame received.\n");
                self.u2f_client.map(|client| client.frame_received());
            }
        }

    }

    /// Handle all endpoint 0 events; clear pending interrupt flags,
    /// swap buffers if needed, then either stall, dispatch to
    /// `handle_setup`, or dispatch to `expect_setup_packet` depending
    /// on whether the setup packet is ready.
    fn handle_endpoint0_events(&self, out_interrupt: bool, in_interrupt: bool) {
        let ep_out = &self.registers.out_endpoints[0];
        let ep_out_interrupts = ep_out.interrupt.extract();
        if out_interrupt {
            ep_out.interrupt.set(ep_out_interrupts.get());
        }

        let ep_in = &self.registers.in_endpoints[0];
        let ep_in_interrupts = ep_in.interrupt.extract();
        if in_interrupt { // Clear interrupts
            ep_in.interrupt.set(ep_in_interrupts.get());
        }

        // If the transfer is compelte (XferCompl), swap which EP0
        // OUT descriptor to use so stack can immediately receive again.
        if out_interrupt && ep_out_interrupts.is_set(OutEndpointInterruptMask::TransferCompleted) {
            self.swap_ep0_out_descriptors();
        }

        let transfer_type = TableCase::decode_interrupt(ep_out_interrupts);
        let flags = self.ep0_out_descriptors
            .map(|descs| descs[self.last_ep0_out_idx.get()].flags)
            .unwrap();

        let setup_ready = flags & DescFlag::SETUP_READY == DescFlag::SETUP_READY;
        control_debug!("USB: handle endpoint 0, transfer type: {:?}, setup_ready: {}\n", transfer_type, setup_ready);

        match self.state.get() {
            USBState::WaitingForSetupPacket => {
                control_debug!("USB: waiting for setup\n");
                if transfer_type == TableCase::A || transfer_type == TableCase::C {
                    if setup_ready {
                        self.handle_setup(transfer_type);
                    } else {
                        control_debug!("Unhandled USB event out:{:#x} in:{:#x} ",
                                       ep_out_interrupts.get(),
                                       ep_in_interrupts.get());
                        control_debug!("flags: \n");
                        if (flags & DescFlag::LAST) == DescFlag::LAST                {control_debug!(" +LAST\n");}
                        if (flags & DescFlag::SHORT) == DescFlag::SHORT              {control_debug!(" +SHORT\n");}
                        if (flags & DescFlag::IOC) == DescFlag::IOC                  {control_debug!(" +IOC\n");}
                        if (flags & DescFlag::SETUP_READY) == DescFlag::SETUP_READY  {control_debug!(" +SETUP_READY\n");}
                        if (flags & DescFlag::HOST_BUSY) == DescFlag::HOST_READY     {control_debug!(" +HOST_READY\n");}
                        if (flags & DescFlag::HOST_BUSY) == DescFlag::DMA_BUSY       {control_debug!(" +DMA_BUSY\n");}
                        if (flags & DescFlag::HOST_BUSY) == DescFlag::DMA_DONE       {control_debug!(" +DMA_DONE\n");}
                        if (flags & DescFlag::HOST_BUSY) == DescFlag::HOST_BUSY      {control_debug!(" +HOST_BUSY\n");}
                        control_debug!("Non-setup packet received before setup: host thinks we've already enumerated.");

                        self.handle_unexpected_packet();
                    }
                } else if transfer_type == TableCase::B {
                    // Only happens when we're stalling, so just keep waiting
                    // for a SETUP
                    self.stall_both_fifos();
                }
            }
            USBState::DataStageIn => {
                control_debug!("USB: state is data stage in\n");
                if in_interrupt &&
                    ep_in_interrupts.is_set(InEndpointInterruptMask::TransferCompleted) {
                        self.registers.in_endpoints[0].control.write(EndpointControl::Enable::SET);
                    }

                if out_interrupt {
                    if transfer_type == TableCase::B {
                        // IN detected
                        self.registers.in_endpoints[0].control.write(EndpointControl::Enable::SET +
                                                                     EndpointControl::ClearNak::SET);
                        self.registers.out_endpoints[0].control.write(EndpointControl::Enable::SET +
                                                                      EndpointControl::ClearNak::SET);
                    } else if transfer_type == TableCase::A || transfer_type == TableCase::C {
                        if setup_ready {
                            self.handle_setup(transfer_type);
                        } else {
                            self.expect_setup_packet();
                        }
                    }
                }
            }
            USBState::NoDataStage => {
                if in_interrupt &&
                    ep_in_interrupts.is_set(InEndpointInterruptMask::TransferCompleted) {
                        self.registers.in_endpoints[0].control.write(EndpointControl::Enable::SET);
                }

                if out_interrupt {
                    if transfer_type == TableCase::B {
                        // IN detected
                        self.registers.in_endpoints[0].control.write(EndpointControl::Enable::SET +
                                                                     EndpointControl::ClearNak::SET);
                        self.registers.out_endpoints[0].control.write(EndpointControl::Enable::SET +
                                                                      EndpointControl::ClearNak::SET);
                    } else if transfer_type == TableCase::A || transfer_type == TableCase::C {
                        if setup_ready {
                            self.handle_setup(transfer_type);
                        } else {
                            self.expect_setup_packet();
                        }
                    } else {
                        self.expect_setup_packet();
                    }
                }
            }
        }
    }

    /// Handle a SETUP packet to endpoint 0 OUT, dispatching to a
    /// helper function depending on what kind of a request it is.
    /// `transfer_type` is the `TableCase` found by inspecting
    /// endpoint-0's interrupt register. Based on the direction of the
    /// request and data size, this function calls one of
    ///   - handle_standard_device_to_host: getting status, descriptors, etc.,
    ///   - handle_standard_host_to_device: none supported yet
    ///   - handle_standard_no_data_phase: setting configuration and address,
    ///   - handle_class_interface_to_host: getting HID report descriptor, or
    ///   - handle_class_host_to_interface: setting idle interval.
    fn handle_setup(&self, transfer_type: TableCase) {
        // Assuming `ep0_out_buffers` was properly set in `init`, this will
        // always succeed.
        control_debug!("Handle setup, case {:?}\n", transfer_type);
        self.ep0_out_buffers.get().map(|bufs| {
            let request = SetupRequest::new(&bufs[self.last_ep0_out_idx.get()]);
            control_debug!("  - type={:?} recip={:?} dir={:?} request={:?}\n", request.req_type(), request.recipient(), request.data_direction(), request.request());

            if request.req_type() == SetupRequestClass::Standard {
                if request.recipient() == SetupRecipient::Device {
                    control_debug!("Standard request on device.\n");
                    if request.data_direction() == SetupDirection::DeviceToHost {
                        self.handle_standard_device_to_host(transfer_type, &request);
                    } else if request.w_length > 0 { // Data requested
                        self.handle_standard_host_to_device(transfer_type, &request);
                    } else { // No data requested
                        self.handle_standard_no_data_phase(transfer_type, &request);
                    }
                } else if request.recipient() == SetupRecipient::Interface {
                    control_debug!("Standard request on interface.\n");
                    if request.data_direction() == SetupDirection::DeviceToHost {
                        self.handle_standard_interface_to_host(transfer_type, &request);
                    } else {
                        self.handle_standard_host_to_interface(transfer_type, &request);
                    }
                }
            } else if request.req_type() == SetupRequestClass::Class && request.recipient() == SetupRecipient::Interface {
                if request.data_direction() == SetupDirection::DeviceToHost {
                    self.handle_class_interface_to_host(transfer_type, &request);
                } else {
                    self.handle_class_host_to_interface(transfer_type, &request);
                }
            } else {
                control_debug!("  - unknown case.\n");
            }
        });
    }

    fn handle_standard_host_to_device(&self, _transfer_type: TableCase, _request: &SetupRequest) {
        // TODO(alevy): don't support any of these yet...
        unimplemented!();
    }

    /// Handles requests for data from device to host, including the device descriptor,
    /// configuration descriptors, interface descriptors, string descriptors, the current
    /// configuration and the device status.
    fn handle_standard_device_to_host(&self, transfer_type: TableCase, request: &SetupRequest) {
        use self::types::SetupRequestType::*;
        use self::serialize::Serialize;
        match request.request() {
            GetDescriptor => {
                let descriptor_type: u32 = (request.w_value >> 8) as u32;
                match descriptor_type {
                    GET_DESCRIPTOR_DEVICE => {
                        let mut len = self.ep0_in_buffers.map(|buf| {
                            self.generate_device_descriptor().serialize(buf)
                        }).unwrap_or(0);

                        len = ::core::cmp::min(len, request.w_length as usize);
                        self.ep0_in_descriptors.map(|descs| {
                            descs[0].flags = (DescFlag::HOST_READY |
                                              DescFlag::LAST |
                                              DescFlag::SHORT |
                                              DescFlag::IOC).bytes(len as u16);
                        });

                        control_debug!("Trying to send device descriptor.\n");
                        self.expect_data_phase_in(transfer_type);
                    },
                    GET_DESCRIPTOR_CONFIGURATION => {
                        let mut len = 0;
                        self.ep0_in_buffers.map(|buf| {
                            self.configuration_descriptor.map(|desc| {
                                len = self.get_configuration_total_length();
                                for i in 0..16 {
                                    buf[i] = desc[4 * i + 0] as u32 |
                                             (desc[4 * i + 1] as u32) << 8 |
                                             (desc[4 * i + 2] as u32) << 16 |
                                             (desc[4 * i + 3] as u32) << 24;
                                }
                            });
                        });
                        control_debug!("USB: Trying to send configuration descriptor, len {}\n  ", len);
                        len = ::core::cmp::min(len, request.w_length);
                        self.ep0_in_descriptors.map(|descs| {
                            descs[0].flags = (DescFlag::HOST_READY |
                                              DescFlag::LAST |
                                              DescFlag::SHORT |
                                              DescFlag::IOC).bytes(len as u16);
                        });
                        self.expect_data_phase_in(transfer_type);
                    },
                    GET_DESCRIPTOR_INTERFACE => {
                        let i = InterfaceDescriptor::new(STRING_INTERFACE2, 0, 0x03, 0, 0);
                        let mut len = 0;
                        self.ep0_in_buffers.map(|buf| {
                            len = i.into_u32_buf(buf);
                        });
                        len = ::core::cmp::min(len, request.w_length as usize);
                        self.ep0_in_descriptors.map(|descs| {
                            descs[0].flags = (DescFlag::HOST_READY |
                                              DescFlag::LAST |
                                              DescFlag::SHORT |
                                              DescFlag::IOC).bytes(len as u16);
                        });
                        self.expect_data_phase_in(transfer_type);
                    },
                    GET_DESCRIPTOR_DEVICE_QUALIFIER => {
                        control_debug!("Trying to send device qualifier: stall both fifos.\n");
                        self.stall_both_fifos();
                    }
                    GET_DESCRIPTOR_STRING => {
                        let index = (request.w_value & 0xff) as usize;
                        self.strings.map(|strs| {
                            let str = &strs[index];
                            let mut len = 0;
                            self.ep0_in_buffers.map(|buf| {
                                len = str.into_u32_buf(buf);
                            });
                            len = ::core::cmp::min(len, request.w_length as usize);
                            self.ep0_in_descriptors.map(|descs| {
                                descs[0].flags = (DescFlag::HOST_READY |
                                                  DescFlag::LAST |
                                                  DescFlag::SHORT |
                                                  DescFlag::IOC).bytes(len as u16);
                            });
                            self.expect_data_phase_in(transfer_type);

                            control_debug!("USB: requesting string descriptor {}, len: {}: {:?}", index, len, str);
                        });
                    }
                    _ => {
                        control_debug!("USB: unhandled setup descriptor type: {}", descriptor_type);
                        self.handle_unexpected_packet();
                    }
                }
            }
            GetConfiguration => {
                let mut len = self.ep0_in_buffers
                    .map(|buf| self.configuration_current_value.get().serialize(buf))
                    .unwrap_or(0);

                len = ::core::cmp::min(len, request.w_length as usize);
                self.ep0_in_descriptors.map(|descs| {
                    descs[0].flags = (DescFlag::HOST_READY | DescFlag::LAST |
                                      DescFlag::SHORT | DescFlag::IOC).bytes(len as u16);
                });
                self.expect_data_phase_in(transfer_type);
            }
            GetStatus => {
                self.ep0_in_buffers.map(|buf| {
                    buf[0] = 0x0;
                });
                self.ep0_in_descriptors.map(|descs| {
                    descs[0].flags = (DescFlag::HOST_READY | DescFlag::LAST |
                                      DescFlag::SHORT | DescFlag::IOC)
                        .bytes(2);
                });
                self.expect_status_phase_in(transfer_type);
            }
            _ => {
                self.handle_unexpected_packet();
            }
        }
    }



    /// Responds to a SETUP message destined to an interface. Currently
    /// only handles GetDescriptor requests for Report descriptors.
    fn handle_standard_interface_to_host(&self, transfer_type: TableCase, request: &SetupRequest) {
        control_debug!("Handle setup interface, device to host.\n");
        let request_type = request.request();
        match request_type {
            SetupRequestType::GetDescriptor => {
                let value      = request.value();
                let descriptor = Descriptor::from_u8((value >> 8) as u8);
                let _index      = (value & 0xff) as u8;
                let len        = request.length() as usize;
                control_debug!("  - Descriptor: {:?}, index: {}, length: {}\n", descriptor, _index, len);
                match descriptor {
                    Descriptor::Report => {
                        if U2F_REPORT_DESCRIPTOR.len() != len {
                            control_debug!("Requested report of length {} but length is {}", request.length(), U2F_REPORT_DESCRIPTOR.len());
                            self.handle_bad_packet();
                        }

                        self.ep0_in_buffers.map(|buf| {
                            for i in 0..len {
                                if (i % 4) == 0 {
                                    buf[i / 4] = (U2F_REPORT_DESCRIPTOR[i] as u32) << ((i % 4) * 8);
                                } else {
                                    buf[i / 4] |= (U2F_REPORT_DESCRIPTOR[i] as u32) << ((i % 4) * 8);
                                }
                            }
                            self.ep0_in_descriptors.map(|descs| {
                                descs[0].flags = (DescFlag::HOST_READY |
                                                  DescFlag::LAST |
                                                  DescFlag::SHORT |
                                                  DescFlag::IOC).bytes(len as u16);
                            });
                            self.expect_data_phase_in(transfer_type);
                        });
                    },
                    _ => {
                        control_debug!("Interface device to host, unhandled request");
                        self.handle_unexpected_packet();
                    }
                }
            },
            _ => {
                control_debug!("Interface device to host, unhandled request: {:?}", request_type);
                self.handle_unexpected_packet();
            }
        }
    }

    /// Handles a setup message to an interface, host-to-device
    /// communication.  Not supported.
    fn handle_standard_host_to_interface(&self, _transfer_type: TableCase, _request: &SetupRequest) {
        control_debug!("Unhandled setup: interface, host to device!");
        self.handle_unexpected_packet();
    }

    /// Handles a setup message to a class, device-to-host
    /// communication. Not supported.
    fn handle_class_interface_to_host(&self, _transfer_type: TableCase, _request: &SetupRequest) {
        control_debug!("Unhandled setup: class, device to host.!");
        self.handle_unexpected_packet();
    }

    /// Handles a setup message to a class, host-to-device
    /// communication.  Currently supports only SetIdle commands.
    fn handle_class_host_to_interface(&self, _transfer_type: TableCase, request: &SetupRequest) {
        use self::types::SetupClassRequestType;
        control_debug!("Handle setup class, host to device.\n");
        match request.class_request() {
            SetupClassRequestType::SetIdle => {
                let val = request.value();
                let _interval: u8 = (val & 0xff) as u8;
                let _id: u8 = (val >> 8) as u8;
                control_debug!("SetIdle: {} to {}, stall fifos.", _id, _interval);
                self.stall_both_fifos();
            },
            _ => {
                self.handle_unexpected_packet();
            }
        }
    }


    /// Handles requests with no accompanying data phase. This includes simple commands
    /// like setting the device address or its which of its configurations to use.
    fn handle_standard_no_data_phase(&self, transfer_type: TableCase, request: &SetupRequest) {
        use self::types::SetupRequestType::*;
        control_debug!(" - setup (no data): {:?}\n", request.request());
        match request.request() {
            GetStatus => {
                control_debug!("USB: GET_STATUS no data setup packet.");
                self.handle_unexpected_packet();
            }
            SetAddress => {
                control_debug!("Setting address: {:#x}.\n", request.w_value & 0x7f);
                // Even though USB wants the address to be set after the
                // IN packet handshake, the hardware knows to wait, so
                // we should just set it now.
                let new_addr = (request.w_value & 0x7f) as u32;
                self.registers.device_config.modify(DeviceConfig::DeviceAddress.val(new_addr));
                self.setup_u2f_descriptors(); // Need to activate EP1 after SetAddress
                self.expect_status_phase_in(transfer_type);
            }
            SetConfiguration => {
                control_debug!("SetConfiguration: {:?} Type {:?} transfer\n", request.w_value, transfer_type);
                self.configuration_current_value.set(request.w_value as u8);
                self.expect_status_phase_in(transfer_type);
            }
            _ => {
                control_debug!("USB: unhandled no data setup packet {}", request.b_request as u8);
                self.handle_unexpected_packet();
            }
        }
    }


    /// Send data to the host over endpoint 0; assumes that IN0 buffers and descriptors
    /// have already been prepared.
    fn expect_data_phase_in(&self, transfer_type: TableCase) {
        self.state.set(USBState::DataStageIn);
        control_debug!("USB: expect_data_phase_in, case: {:?}\n", transfer_type);
        self.ep0_in_descriptors.map(|descs| {
            // 2. Flush fifos
            self.flush_tx_fifo(0);

            // 3. Set EP0 in DMA
            self.registers.in_endpoints[0].dma_address.set(&descs[0]);
            control_debug!("USB: expect_data_phase_in: endpoint 0 descriptor: flags={:08x} addr={:08x} \n", descs[0].flags.0, descs[0].addr);

            // If we clear the NAK (write CNAK) then this responds to
            // a non-setup packet, leading to failure as the code
            // needs to first respond to a setup packet.
            if transfer_type == TableCase::C {
                self.registers.in_endpoints[0].control.write(EndpointControl::Enable::SET +
                                                             EndpointControl::ClearNak::SET);
            } else {
                self.registers.in_endpoints[0].control.write(EndpointControl::Enable::SET);
            }

            self.ep0_out_descriptors.map(|descs| {
                descs[self.next_ep0_out_idx.get()].flags =
                    (DescFlag::HOST_READY | DescFlag::LAST | DescFlag::IOC).bytes(64);
            });

            // If we clear the NAK (write CNAK) then this responds to
            // a non-setup packet, leading to failure as the code
            // needs to first respond to a setup packet.
            if transfer_type == TableCase::C {
                self.registers.out_endpoints[0].control.write(EndpointControl::Enable::SET +
                                                              EndpointControl::ClearNak::SET);
            } else {
                self.registers.out_endpoints[0].control.write(EndpointControl::Enable::SET);
            }
            control_debug!("Registering for IN0 and OUT0 interrupts.\n");
            self.registers
                .device_all_ep_interrupt_mask
                .modify(AllEndpointInterrupt::IN0::SET +
                        AllEndpointInterrupt::OUT0::SET);
        });
    }

    /// Setup endpoint 0 for a status phase with no data phase.
    fn expect_status_phase_in(&self, transfer_type: TableCase) {
        self.state.set(USBState::NoDataStage);
        control_debug!("USB: expect_status_phase_in, case: {:?}\n", transfer_type);

        self.ep0_in_descriptors.map(|descs| {
            // 1. Expect a zero-length in for the status phase
            // IOC, Last, Length 0, SP
            self.ep0_in_buffers.map(|buf| {
                // Address doesn't matter since length is zero
                descs[0].addr = buf.as_ptr() as usize;
            });
            descs[0].flags =
                (DescFlag::HOST_READY | DescFlag::LAST | DescFlag::SHORT | DescFlag::IOC).bytes(0);

            // 2. Flush fifos
            self.flush_tx_fifo(0);

            // 3. Set EP0 in DMA
            self.registers.in_endpoints[0].dma_address.set(&descs[0]);

            if transfer_type == TableCase::C {
                self.registers.in_endpoints[0].control.write(EndpointControl::Enable::SET +
                                                             EndpointControl::ClearNak::SET);

            } else {
                self.registers.in_endpoints[0].control.write(EndpointControl::Enable::SET);
            }


            self.ep0_out_descriptors.map(|descs| {
                descs[self.next_ep0_out_idx.get()].flags =
                    (DescFlag::HOST_READY | DescFlag::LAST | DescFlag::IOC).bytes(64);
            });

            if transfer_type == TableCase::C {
                self.registers.out_endpoints[0].control.write(EndpointControl::Enable::SET +
                                                              EndpointControl::ClearNak::SET);
            } else {
                self.registers.out_endpoints[0].control.write(EndpointControl::Enable::SET);
            }

            let mask = self.registers.device_all_ep_interrupt_mask.extract();
            self.registers
                .device_all_ep_interrupt_mask
                .modify_no_read(mask,
                                AllEndpointInterrupt::IN0::SET +
                                AllEndpointInterrupt::OUT0::SET);
        });
    }

    /// Flush endpoint 0's RX FIFO
    ///
    /// # Safety
    ///
    /// Only call this when  transaction is not underway and data from this FIFO
    /// is not being copied.
    fn flush_rx_fifo(&self) {
        self.registers.reset.write(Reset::TxFifoFlush::SET); // TxFFlsh

        // Wait for TxFFlsh to clear
        while self.registers.reset.is_set(Reset::TxFifoFlush) {}
    }

    /// Flush one or all endpoint TX FIFOs.
    ///
    /// `fifo_num` is 0x0-0xF for a particular fifo, or 0x10 for all fifos
    ///
    /// # Safety
    ///
    /// Only call this when  transaction is not underway and data from this FIFO
    /// is not being copied.
    fn flush_tx_fifo(&self, fifo_num: u8) {
        let fifo_field = match fifo_num {
            0  => Reset::TxFifoNumber::Fifo0,
            1  => Reset::TxFifoNumber::Fifo1,
            2  => Reset::TxFifoNumber::Fifo2,
            3  => Reset::TxFifoNumber::Fifo3,
            4  => Reset::TxFifoNumber::Fifo4,
            5  => Reset::TxFifoNumber::Fifo5,
            6  => Reset::TxFifoNumber::Fifo6,
            7  => Reset::TxFifoNumber::Fifo7,
            8  => Reset::TxFifoNumber::Fifo8,
            9  => Reset::TxFifoNumber::Fifo9,
            10 => Reset::TxFifoNumber::Fifo10,
            11 => Reset::TxFifoNumber::Fifo11,
            12 => Reset::TxFifoNumber::Fifo12,
            13 => Reset::TxFifoNumber::Fifo13,
            14 => Reset::TxFifoNumber::Fifo14,
            15 => Reset::TxFifoNumber::Fifo15,
            16 => Reset::TxFifoNumber::AllFifos,
            _  => Reset::TxFifoNumber::AllFifos, // Should Panic, or make param typed
        };
        self.registers.reset.write(Reset::TxFifoFlush::SET + fifo_field);

        // Wait for TxFFlsh to clear
        while self.registers.reset.is_set(Reset::TxFifoFlush) {}
    }

    /// Initialize hardware data fifos
    // The constants matter for correct operation and are dependent on settings
    // in the coreConsultant. If the value is too large, the transmit_fifo_size
    // register will end up being 0, which is too small to transfer anything.
    //
    // In our case, I'm not sure what the maximum size is, but `TX_FIFO_SIZE` of
    // 32 work and 512 is too large.
    fn setup_data_fifos(&self) {
        // 3. Set up data FIFO RAM
        self.registers.receive_fifo_size.set(RX_FIFO_SIZE as u32 & 0xffff);
        self.registers
            .transmit_fifo_size
            .set(((TX_FIFO_SIZE as u32) << 16) | ((RX_FIFO_SIZE as u32) & 0xffff));
        for (i, d) in self.registers.device_in_ep_tx_fifo_size.iter().enumerate() {
            let i = i as u16;
            d.set(((TX_FIFO_SIZE as u32) << 16) | (RX_FIFO_SIZE + i * TX_FIFO_SIZE) as u32);
        }

        self.flush_tx_fifo(0x10);
        self.flush_rx_fifo();

    }

    /// Generate the binary representation of the configuration descriptor for the
    /// device. This is currently hardcoded to include:
    ///   - The U2F Interface Descriptor
    ///   - The HID Device Descriptor
    ///   - The EP1 out Endpoint Descriptor (U2F)
    ///   - The EP1 in Endpoint Descriptor (U2F)
    ///   - The Shell Device Descriptor
    fn generate_full_configuration_descriptor(&self) {
        self.configuration_descriptor.map(|desc| {

            let mut config = ConfigurationDescriptor::new(1, STRING_PLATFORM, 50);

            let attributes_u2f_in = EndpointAttributes {
                transfer: EndpointTransferType::Interrupt,
                synchronization: EndpointSynchronizationType::None,
                usage: EndpointUsageType::Data,
            };
            let attributes_u2f_out = EndpointAttributes {
                transfer: EndpointTransferType::Interrupt,
                synchronization: EndpointSynchronizationType::None,
                usage: EndpointUsageType::Data,
            };

            let u2f = InterfaceDescriptor::new(STRING_INTERFACE2, 0, 3, 0, 0);
            let hid = HidDeviceDescriptor::new();
            let ep1out = EndpointDescriptor::new(0x01, attributes_u2f_out, 2);
            let ep1in  = EndpointDescriptor::new(0x81, attributes_u2f_in, 2);

            let mut size: usize = config.length();
            size += u2f.into_u8_buf(&mut desc[size..size + u2f.length()]);
            size += hid.into_u8_buf(&mut desc[size..size + hid.length()]);
            size += ep1out.into_u8_buf(&mut desc[size..size + ep1out.length()]);
            size += ep1in.into_u8_buf(&mut desc[size..size + ep1in.length()]);

            // In case we want to start including a shell like the normal gnubby.
            // Note this requires changing config to have 2 interfaces, not 1.
            /*let attributes_shell_in = EndpointAttributes {
                transfer: EndpointTransferType::Bulk,
                synchronization: EndpointSynchronizationType::None,
                usage: EndpointUsageType::Data,
            };
            let attributes_shell_out = EndpointAttributes {
                transfer: EndpointTransferType::Bulk,
                synchronization: EndpointSynchronizationType::None,
                usage: EndpointUsageType::Data,
            };
            let shell = InterfaceDescriptor::new(STRING_INTERFACE1, 1, 0xFF, 80, 1);
            let ep2in  = EndpointDescriptor::new(0x82, attributes_shell_in, 10);
            let ep2out = EndpointDescriptor::new(0x02, attributes_shell_out, 0);
            size += shell.into_u8_buf(&mut desc[size..size + shell.length()]);
            size += ep2in.into_u8_buf(&mut desc[size..size + ep2in.length()]);
            size += ep2out.into_u8_buf(&mut desc[size..size + ep2out.length()]);*/

            config.set_total_length(size as u16);
            config.into_u8_buf(&mut desc[0..config.length()]);
            self.set_configuration_total_length(size as u16);
        });
    }

    pub fn set_configuration_total_length(&self, length: u16) {
        self.configuration_total_length.set(length);
    }

    pub fn get_configuration_total_length(&self) -> u16 {
        self.configuration_total_length.get()
    }

    /// Stalls both the IN and OUT endpoints for endpoint 0.
    //
    // A STALL condition indicates that an endpoint is unable to
    // transmit or receive data.  STALLing when waiting for a SETUP
    // message forces the host to send a new SETUP. This can be used to
    // indicate the request wasn't understood or needs to be resent.
    fn stall_both_fifos(&self) {
        control_debug!("USB: WaitingForSetupPacket in stall_both_fifos.\n");
        self.state.set(USBState::WaitingForSetupPacket);
        self.ep0_out_descriptors.map(|descs| {
            descs[self.next_ep0_out_idx.get()].flags = (DescFlag::LAST | DescFlag::IOC).bytes(64);
        });

        // Enable OUT and disable IN interrupts
        self.registers.device_all_ep_interrupt_mask.modify(AllEndpointInterrupt::OUT0::SET + AllEndpointInterrupt::IN0::CLEAR);

        self.registers.out_endpoints[0].control.write(EndpointControl::Enable::SET +
                                                      EndpointControl::Stall::SET);
        self.flush_tx_fifo(0);
        self.registers.in_endpoints[0].control.write(EndpointControl::Enable::SET +
                                                     EndpointControl::Stall::SET);
    }

    fn handle_unexpected_packet(&self) {
        // USB 2.0 specification, 9.2.7 "Request Error"
        // "When a request is received by a device that is not defined for the device,
        // is inappropriate for the current setting of the device, or has values that
        // are not compatible with the request, then a Request Error exists.
        // The device deals with the Request Error by returning a STALL PID in response
        // to the next Data stage transaction or in the Status stage of the message.
        // It is preferred that the STALL PID be returned at the next Data stage transaction,
        // as this avoids unnecessary bus activity."
        self.stall_both_fifos();
    }

    fn handle_bad_packet(&self) {
        // USB 2.0 specification, 9.2.7 "Request Error"
        // "When a request is received by a device that is not defined for the device,
        // is inappropriate for the current setting of the device, or has values that
        // are not compatible with the request, then a Request Error exists.
        // The device deals with the Request Error by returning a STALL PID in response
        // to the next Data stage transaction or in the Status stage of the message.
        // It is preferred that the STALL PID be returned at the next Data stage transaction,
        // as this avoids unnecessary bus activity."
        self.stall_both_fifos();
    }

    // Helper function which swaps which EP0 out descriptor is set up
    // to receive so software can receive a new packet while
    // processing the current one.
    fn swap_ep0_out_descriptors(&self) {
        self.ep0_out_descriptors.map(|descs| {
            let mut noi = self.next_ep0_out_idx.get();
            self.last_ep0_out_idx.set(noi);
            noi = (noi + 1) % descs.len();
            self.next_ep0_out_idx.set(noi);
            self.registers.out_endpoints[0].dma_address.set(&descs[noi]);
        });
    }

    // Construct a USB Device Descriptor from the configuration parameters
    // of the USB driver.
    fn generate_device_descriptor(&self) -> DeviceDescriptor {
        DeviceDescriptor {
            b_length: 18,
            b_descriptor_type: 1,
            bcd_usb: 0x0200,
            b_device_class: self.device_class.get(),
            b_device_sub_class: 0x00,
            b_device_protocol: 0x00,
            b_max_packet_size0: MAX_PACKET_SIZE as u8,
            id_vendor: self.vendor_id.get(),
            id_product: self.product_id.get(),
            bcd_device: 0x0100,
            i_manufacturer: STRING_VENDOR,
            i_product: STRING_BOARD,
            i_serial_number: STRING_LANG,
            b_num_configurations: 1,
        }
    }


    /// Initialize the USB driver in device mode, so it can be begin
    /// communicating with a connected host.
    pub fn init(&self,
                ep0_out_descriptors: &'static mut [DMADescriptor; EP0_OUT_BUFFER_COUNT],
                ep0_out_buffers: &'static mut [[u32; 16]; EP0_OUT_BUFFER_COUNT],
                ep0_in_descriptors: &'static mut [DMADescriptor; EP0_IN_BUFFER_COUNT],
                ep0_in_buffers: &'static mut [u32; 16 * 4],
                ep1_out_descriptor: &'static mut DMADescriptor,
                ep1_out_buffer: &'static mut [u32; 16],
                ep1_in_descriptor: &'static mut DMADescriptor,
                ep1_in_buffer: &'static mut [u32; 16],
                configuration_buffer: &'static mut [u8; 64],
                phy: PHY,
                device_class: Option<u8>,
                vendor_id: Option<u16>,
                product_id: Option<u16>,
                strings: &'static mut [StringDescriptor]) {
        self.ep0_out_descriptors.replace(ep0_out_descriptors);
        self.ep0_out_buffers.set(Some(ep0_out_buffers));
        self.ep0_in_descriptors.replace(ep0_in_descriptors);
        self.ep0_in_buffers.replace(ep0_in_buffers);
        self.ep1_out_descriptor.replace(ep1_out_descriptor);
        self.ep1_out_buffer.set(Some(ep1_out_buffer));
        self.ep1_in_descriptor.replace(ep1_in_descriptor);
        self.ep1_in_buffer.replace(ep1_in_buffer);
        self.configuration_descriptor.replace(configuration_buffer);
        self.strings.replace(strings);

        if let Some(dclass) = device_class {
            self.device_class.set(dclass);
        }

        if let Some(vid) = vendor_id {
            self.vendor_id.set(vid);
        }

        if let Some(pid) = product_id {
            self.product_id.set(pid);
        }

        self.generate_full_configuration_descriptor();

        self.core_clock.enable();
        self.timer_clock.enable();

        self.registers.interrupt_mask.set(0);
        self.registers.device_all_ep_interrupt_mask.set(0);
        self.registers.device_in_ep_interrupt_mask.set(0);
        self.registers.device_out_ep_interrupt_mask.set(0);

        // This code below still needs significant cleanup -pal
        let sel_phy = match phy {
            PHY::A => 0b100, // USB PHY0
            PHY::B => 0b101, // USB PHY1
        };
        // Select PHY A
        self.registers.gpio.write(Gpio::GpoOperation::Write +
                                  Gpio::GpoRegister.val(0) +
                                  Gpio::GpoValue.val(sel_phy as u32));

        // Configure the chip
        // Timing values copied from Cr50 C reference code
        self.registers.configuration.write(UsbConfiguration::PhySelect::Usb11FullSpeed +
                                           UsbConfiguration::FullSpeedSerialInterfaceSelect::Unidirectional6Pin +
                                           UsbConfiguration::UsbTurnaroundTime.val(14) +
                                           UsbConfiguration::TimeoutCalibration.val(7));

        // Soft reset
        self.soft_reset();

        // Configure the chip
        self.registers.configuration.write(UsbConfiguration::PhySelect::Usb11FullSpeed +
                                           UsbConfiguration::FullSpeedSerialInterfaceSelect::Unidirectional6Pin +
                                           UsbConfiguration::UsbTurnaroundTime.val(14) +
                                           UsbConfiguration::TimeoutCalibration.val(7));

        // === Begin Core Initialization ==//

        // We should be reading `user_hw_config` registers to find out about the
        // hardware configuration (which endpoints are in/out, OTG capable,
        // etc). Skip that for now and just make whatever assumption CR50 is
        // making.

        // Set the following parameters:
        //   * Enable DMA Mode
        //   * Global unmask interrupts
        //   * Interrupt on Non-Periodic TxFIFO completely empty
        // _Don't_ set:
        //   * Periodic TxFIFO interrupt on empty (only valid in device mode)
        //   * AHB Burst length (defaults to 1 word)
        self.registers.ahb_config.write(AhbConfig::GlobalInterruptMask::SET +
                                        AhbConfig::DmaEnable::SET +
                                        AhbConfig::NonPeriodicTxFifoEmptyLevel::SET);

        // Set Soft Disconnect bit to make sure we're in disconnected state
        self.registers.device_control.modify(DeviceControl::SoftDisconnect::SET);

        // The datasheet says to unmask OTG and Mode Mismatch interrupts, but
        // we don't support anything but device mode for now, so let's skip
        // handling that
        //
        // If we're right, then
        // `self.registers.interrupt_status.get() & 1 == 0`
        //

        // === Done with core initialization ==//

        // ===  Begin Device Initialization  ==//

        self.registers.device_config.modify(DeviceConfig::DeviceSpeed::Full1 +
                                            DeviceConfig::EnableScatterGatherDMAInDeviceMode::SET);
        // 0 << 2     | // Non-zero-length Status: send packet to application
        // 0b00 << 11 | // Periodic frame interval: 80%

        // We would set the device threshold control register here, but I don't
        // think we enable thresholding.

        self.setup_data_fifos();

        // Clear any pending interrupts
        for endpoint in self.registers.out_endpoints.iter() {
            endpoint.interrupt.set(!0);
        }
        for endpoint in self.registers.in_endpoints.iter() {
            endpoint.interrupt.set(!0);
        }
        self.registers.interrupt_status.set(!0);

        // Unmask some endpoint interrupts
        //    Device OUT SETUP & XferCompl
        self.registers.device_out_ep_interrupt_mask.write(OutEndpointInterruptMask::TransferCompleted::SET +
                                                          OutEndpointInterruptMask::EndpointDisabled::SET +
                                                          OutEndpointInterruptMask::SetupPhaseDone::SET);
        //    Device IN XferCompl & TimeOut
        self.registers.device_in_ep_interrupt_mask.write(InEndpointInterruptMask::TransferCompleted::SET +
                                                         InEndpointInterruptMask::EndpointDisabled::SET);

        // To set ourselves up for processing the state machine through interrupts,
        // unmask:
        //
        //   * USB Reset
        //   * Enumeration Done
        //   * Early Suspend
        //   * USB Suspend
        //   * SOF
        //
        self.registers
            .interrupt_mask
            .write(Interrupt::GlobalOutNak::SET +
                   Interrupt::GlobalInNak::SET +
                   Interrupt::Reset::SET +
                   Interrupt::EnumerationDone::SET +
                   Interrupt::InEndpoints::SET +
                   Interrupt::OutEndpoints::SET +
                   Interrupt::EarlySuspend::SET +
                   Interrupt::Suspend::SET +
                   Interrupt::StartOfFrame::SET);

        // Power on programming done
        self.registers.device_control.modify(DeviceControl::PowerOnProgrammingDone::SET);

        for _ in 0..10000 {
            support::nop();
        }
        self.registers.device_control.modify(DeviceControl::PowerOnProgrammingDone::CLEAR);

        // Clear global NAKs
        self.registers.device_control.modify(DeviceControl::ClearGlobalOutNak::SET +
                                             DeviceControl::ClearGlobalNonPeriodicInNak::SET);

        // Reconnect:
        //  Clear the Soft Disconnect bit to allow the core to issue a connect.
        self.registers.device_control.modify(DeviceControl::SoftDisconnect::CLEAR);
    }

}

/// Implementation of the HID U2F API for the USB device. It assumes
/// that U2F is over endpoint 1.
impl<'a> UsbHidU2f<'a> for USB<'a> {
    fn set_u2f_client(&self, client: &'a dyn UsbHidU2fClient<'a>) {
        self.u2f_client.set(client);
    }

    // Note that this resets the EP1 (U2F) descriptors and buffers;
    // usb_reset() and init_ep0_descriptors() do these operations on the
    // EP0 (control) descriptors and buffers.
    //
    // This method must be called after a SetConfiguration and SetAddress
    // command, to initialize EP1 and enable data transmission.
    fn setup_u2f_descriptors(&self) {
        self.ep1_out_descriptor.map(|out_desc| {
            self.ep1_out_buffer.get().map(|out_buf| {
                out_desc.flags = (DescFlag::LAST |
                                  DescFlag::HOST_READY |
                                  DescFlag::IOC).bytes(U2F_REPORT_SIZE);
                out_desc.addr = out_buf.as_ptr() as usize;
                self.registers.out_endpoints[1].dma_address.set(&out_desc);
            });
        });

        self.ep1_in_descriptor.map(|in_desc| {
            self.ep1_in_buffer.map(|in_buf| {
                in_desc.flags =  DescFlag::LAST | DescFlag::HOST_BUSY | DescFlag::IOC;
                in_desc.addr = in_buf.as_ptr() as usize;
                self.registers.in_endpoints[1].dma_address.set(&in_desc);

            });
        });

        self.ep1_out_descriptor.map(|_out_desc| {
            self.ep1_out_buffer.get().map(|_out_buf| {
                self.registers.out_endpoints[1].control.write(EndpointControl::Enable::SET +
                                                              EndpointControl::ClearNak::SET +
                                                              EndpointControl::UsbActiveEndpoint::SET +
                                                              EndpointControl::EndpointType::Interrupt +
                                                              EndpointControl::MaximumPacketSize.val(U2F_REPORT_SIZE as u32));
            });
        });

        self.ep1_in_descriptor.map(|_in_desc| {
            self.ep1_in_buffer.map(|_in_buf| {
                self.registers.in_endpoints[1].control.write(EndpointControl::UsbActiveEndpoint::SET +
                                                             EndpointControl::TxFifoNumber.val(1) +
                                                             EndpointControl::EndpointType::Interrupt +
                                                             EndpointControl::MaximumPacketSize.val(U2F_REPORT_SIZE as u32));
            });
        });

        self.registers.device_all_ep_interrupt_mask.modify(AllEndpointInterrupt::OUT1::SET + AllEndpointInterrupt::IN1::SET);
    }

    fn force_reconnect(&self) -> ReturnCode {
        panic!("Trying to force reconnect USB EP1\n");
    }

    fn enable_rx(&self) -> ReturnCode {
        self.ep1_enable_rx();
        ReturnCode::SUCCESS
    }

    fn iface_respond(&self) -> ReturnCode {ReturnCode::FAIL}

    fn transmit_ready(&self) -> bool {
        self.ep1_tx_fifo_is_ready()
    }

    fn put_frame(&self, frame: &[u32; 16]) -> ReturnCode {
        data_debug!("U2F: put_frame\n");
        if !self.ep1_tx_fifo_is_ready() {
            data_debug!("Tried to put frame but busy.\n");
            ReturnCode::EBUSY
        } else {
            self.ep1_in_buffer.map(|hardware_buffer| {
                for i in 0..frame.len() {
                    hardware_buffer[i] = frame[i];
                }
            });
            self.ep1_enable_tx();
            data_debug!("Sending frame.\n");
            ReturnCode::SUCCESS
        }
    }

    fn put_slice(&self, slice: &[u8]) -> ReturnCode {
        data_debug!("U2F: put_slice\n");
        if slice.len() > 64 {
            data_debug!("U2F EP1: ERROR: slice too large\n");
            ReturnCode::ESIZE
        } else if !self.ep1_tx_fifo_is_ready() {
            data_debug!("U2F EP1: ERROR: Tried to put slice but busy.\n");
            ReturnCode::EBUSY
        } else {
            self.ep1_in_buffer.map(|hardware_buffer| {
                for (i, c) in slice.iter().enumerate() {
                    let hw_index = i / 4;
                    let byte_index = i % 4;
                    if byte_index == 0 {
                        hardware_buffer[hw_index] = *c as u32;
                    } else {
                        hardware_buffer[hw_index] |= (*c as u32) << (8 * byte_index);
                    }
                }
            });
            self.ep1_enable_tx();
            data_debug!("U2FData: Started slice send.\n");
            data_debug!("U2FData: {} words available.\n", self.registers.in_endpoints[1].tx_fifo_status.get());
            ReturnCode::SUCCESS
        }
    }

    fn get_frame(&self, frame: &mut [u32; 16]) {
        // Unlike the CR52 code, we don't need to disable interrupts,
        // because Tock handles the USB interrupts as bottom halves. -pal
        self.ep1_out_buffer.get().map(|hardware_buffer| {
            for i in 0..16 {
                frame[i] = hardware_buffer[i];
            }
        });
    }

    fn get_slice(&self, slice: &mut [u8]) -> ReturnCode{
        data_debug!("U2F: get_slice\n");
        if slice.len() > 64 {
            ReturnCode::ESIZE
        } else {
            self.ep1_out_buffer.get().map(|hardware_buffer| {
                let len = slice.len();
                for i in 0..len {
                    let hw_index = i / 4;
                    let byte_index = i % 4;
                    slice[i] = ((hardware_buffer[hw_index] >> (8 * byte_index)) & 0xff) as u8;
                }
            });
            ReturnCode::SUCCESS
        }
    }
}

/// Which physical connection to use
pub enum PHY {
    A,
    B,
}

/// Combinations of OUT endpoint interrupts for control transfers denote
/// different transfer cases.
///
/// TableCase encodes the cases from Table 10.7 in the OTG Programming
/// Guide (pages 279-230).
#[derive(Copy,Clone,PartialEq,Eq,Debug)]
pub enum TableCase {
    /// Case A
    ///
    /// * StsPhseRcvd: 0
    /// * SetUp: 0
    /// * XferCompl: 1
    A,   // OUT descriptor updated; check the SR bit to see if Setup or OUT
    /// Case B
    ///
    /// * StsPhseRcvd: 0
    /// * SetUp: 1
    /// * XferCompl: 0
    B,   // Setup Phase Done for previously decoded Setup packet
    /// Case C
    ///
    /// * StsPhseRcvd: 0
    /// * SetUp: 1
    /// * XferCompl: 1
    C,   // OUT descriptor updated for a Setup packet, Setup complete
    /// Case D
    ///
    /// * StsPhseRcvd: 1
    /// * SetUp: 0
    /// * XferCompl: 0
    D,   // Status phase of Control OUT transfer
    /// Case E
    ///
    /// * StsPhseRcvd: 1
    /// * SetUp: 0
    /// * XferCompl: 1
    E,   // OUT descriptor updated; check SR bit to see if Setup or Out.
         // Plus, host is now in Control Write Status phase
}

impl TableCase {
    /// Decodes a value from the OUT endpoint interrupt register.
    ///
    /// Only properly decodes values with the combinations shown in the
    /// programming guide.
    pub fn decode_interrupt(device_out_int: LocalRegisterCopy<u32, OutEndpointInterruptMask::Register>) -> TableCase {
        if device_out_int.is_set(OutEndpointInterruptMask::TransferCompleted) {
            if device_out_int.is_set(OutEndpointInterruptMask::SetupPhaseDone) {
                TableCase::C
            } else if device_out_int.is_set(OutEndpointInterruptMask::StatusPhaseReceived) {
                TableCase::E
            } else {
                TableCase::A
            }
        } else {
            if device_out_int.is_set(OutEndpointInterruptMask::SetupPhaseDone) {
                TableCase::B
            } else {
                TableCase::D
            }
        }
    }
}

fn print_in_endpoint_interrupt_status(status: LocalRegisterCopy<u32, InEndpointInterruptMask::Register>) {
    int_debug!("USB in endpoint interrupt, status: {:08x}\n", status);
    if status.is_set(InEndpointInterruptMask::TransferCompleted)    {data_debug!("  +Transfer complete\n");}
    if status.is_set(InEndpointInterruptMask::EndpointDisabled)    {data_debug!("  +Endpoint disabled\n");}
    if status.is_set(InEndpointInterruptMask::AhbError)            {data_debug!("  +AHB Error\n");}
    if status.is_set(InEndpointInterruptMask::Timeout)             {data_debug!("  +Timeout\n");}
    if status.is_set(InEndpointInterruptMask::InTokenReceivedWhenTxFifoEmpty)      {data_debug!("  +In token received when TX FIFO empty\n");}
    if status.is_set(InEndpointInterruptMask::InTokenEndpointMismatched)           {data_debug!("  +In token endpoint mismatch\n");}
    if status.is_set(InEndpointInterruptMask::InEndpointNakEffective)              {data_debug!("  +In endpoint NAK effective\n");}
    if status.is_set(InEndpointInterruptMask::TxFifoUnderrun)      {data_debug!("  +TXFifo underrun\n");}
    if status.is_set(InEndpointInterruptMask::BufferNotAvailable)  {data_debug!("  +Buff not available\n");}
    if status.is_set(InEndpointInterruptMask::NAK)                 {data_debug!("  +NAK\n");}
}

fn print_usb_interrupt_status(status: LocalRegisterCopy<u32, Interrupt::Register>) {
    int_debug!("USB interrupt, status: {:08x}\n", status.get());
    if status.matches_all(Interrupt::CurrentMode::Host)  {int_debug!("  +Host mode\n");}
    if status.is_set(Interrupt::ModeMismatch)       {int_debug!("  +Mismatch\n");}
    if status.is_set(Interrupt::OTG)                {int_debug!("  +OTG\n");}
    if status.is_set(Interrupt::StartOfFrame)       {int_debug!("  +SOF\n");}
    if status.is_set(Interrupt::RxFifoNotEmpty)     {int_debug!("  +RxFIFO\n");}
    if status.is_set(Interrupt::GlobalInNak)        {int_debug!("  +GlobalInNak\n");}
    if status.is_set(Interrupt::GlobalOutNak)       {int_debug!("  +GlobalOutNak\n");}
    if status.is_set(Interrupt::EarlySuspend)       {int_debug!("  +EarlySuspend\n");}
    if status.is_set(Interrupt::Suspend)            {int_debug!("  +Suspend\n");}
    if status.is_set(Interrupt::Reset)              {int_debug!("  +USB reset\n");}
    if status.is_set(Interrupt::EnumerationDone)    {int_debug!("  +Speed enum done\n");}
    if status.is_set(Interrupt::EndOfPeriodicFrame) {int_debug!("  +EOPF\n");}
    if status.is_set(Interrupt::EndpointMismatch)   {int_debug!("  +Endpoint mismatch\n");}
    if status.is_set(Interrupt::InEndpoints)        {int_debug!("  +IN endpoints\n");}
    if status.is_set(Interrupt::OutEndpoints)       {int_debug!("  +OUT endpoints\n");}
    if status.is_set(Interrupt::IncompleteIsochronousInTransfer)   {int_debug!("  +IN ISOC incomplete\n");}
    if status.is_set(Interrupt::IncompletePeriodicTransfer) {int_debug!("  +Incomp periodic\n");}
    if status.is_set(Interrupt::DataFetchSuspended) {int_debug!("  +Fetch suspend\n");}
    if status.is_set(Interrupt::ResetDetected)      {int_debug!("  +Reset detected\n");}
    if status.is_set(Interrupt::ConnectIDChange)    {int_debug!("  +Connect ID change\n");}
    if status.is_set(Interrupt::DisconnectDetected) {int_debug!("  +Disconnect detected\n");}
    if status.is_set(Interrupt::SessionRequest)     {int_debug!("  +Session request\n");}
    if status.is_set(Interrupt::ResumeWakeup)       {int_debug!("  +Resume/wakeup\n");}
}

/* Statically allocated in-memory state and message buffers.*/


// These are HW, not USB descriptors: they describe the
// current state of hardware for USB endpoints, including
// status flags and a pointer into a data buffer.
pub static mut EP0_OUT_DESCRIPTORS: [DMADescriptor; EP0_OUT_BUFFER_COUNT] = [DMADescriptor {
    flags: DescFlag::HOST_BUSY,
    addr: 0,
}; EP0_OUT_BUFFER_COUNT];
pub static mut EP0_IN_DESCRIPTORS: [DMADescriptor; EP0_IN_BUFFER_COUNT] = [DMADescriptor {
    flags: DescFlag::HOST_BUSY,
    addr: 0,
}; EP0_IN_BUFFER_COUNT];

pub static mut EP0_OUT_BUFFERS: [[u32; EP_BUFFER_SIZE_WORDS]; EP0_OUT_BUFFER_COUNT] =
                                  [[0; EP_BUFFER_SIZE_WORDS]; EP0_OUT_BUFFER_COUNT];
pub static mut EP0_IN_BUFFER: [u32; EP_BUFFER_SIZE_WORDS * EP0_IN_BUFFER_COUNT] =
                                [0; EP_BUFFER_SIZE_WORDS * EP0_IN_BUFFER_COUNT];

pub static mut EP1_OUT_DESCRIPTOR: DMADescriptor = DMADescriptor {flags: DescFlag::HOST_BUSY,
                                                                  addr: 0};
pub static mut EP1_IN_DESCRIPTOR:  DMADescriptor = DMADescriptor {flags: DescFlag::HOST_BUSY,
                                                                  addr: 0};
pub static mut EP1_OUT_BUFFER: [u32; EP_BUFFER_SIZE_WORDS] = [0; EP_BUFFER_SIZE_WORDS];
pub static mut EP1_IN_BUFFER:  [u32; EP_BUFFER_SIZE_WORDS] = [0; EP_BUFFER_SIZE_WORDS];

// Buffer used to store device configuration (descriptors), initialized at startup.
pub static mut CONFIGURATION_BUFFER: [u8; EP_BUFFER_SIZE_BYTES] = [0; EP_BUFFER_SIZE_BYTES];
