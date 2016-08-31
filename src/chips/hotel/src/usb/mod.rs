use pmu::{Clock, PeripheralClock, PeripheralClock1};
use core::cell::Cell;
use core::ops::Deref;
use common::take_cell::TakeCell;

mod constants;
mod registers;

use self::constants::*;
use self::registers::Registers;

pub use self::registers::DMADescriptor;

/// A pointer to statically allocated mutable data such as memory mapped I/O
/// registers.
///
/// This is a simple wrapper around a raw pointer that encapsulates an unsafe
/// dereference in a safe manner. It serve the role of creating a `&'static T`
/// given a raw address and acts similarly to `extern` definitions, except
/// `StaticRef` is subject to module and crate bounderies, while `extern`
/// definitions can be imported anywhere.
///
/// TODO(alevy): move into `common` crate or replace with other mechanism.
struct StaticRef<T> {
    ptr: *const T
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

/// Encodes the current state of the USB driver's state machine
#[derive(Clone,Copy,PartialEq,Eq)]
enum USBState {
	WaitingForSetupPacket,
	DataStageIn,
	NoDataStage,
}

/// USB driver for the Synopsis controller
///
/// The driver operates as a device in Scatter-Gather DMA mode and performs the
/// initial handshake with the host on endpoint 0.
///
/// Scatter-gather mode operates using lists of descriptors which point to
/// memory buffers. Each buffer can hold up to 64 bytes, so a transfer larger
/// than 64 bytes involves multiple descriptors in sequence.
///
/// For endpoint 0, the driver configures two OUT descriptors and 4 IN
/// descriptors.
///
/// Four IN descriptors allows responses up to 256 bytes (64 * 4), which is
/// important for returning the configuration descriptor as one big blob.
///
/// We never expect to receive OUT packets larger than 64 bytes (the maximum
/// each descriptor can handle), but we use two so we can receive the next
/// packet while processing the previous one.
pub struct USB {
    registers: StaticRef<Registers>,

    core_clock: Clock,
    timer_clock: Clock,

    /// Current state of the driver
    state: Cell<USBState>,

    /// Endpoint 0 OUT descriptors
    ///
    /// ## Invariants
    /// The `TakeCell` is never empty after a call to `init`.
    ep0_out_descriptors: TakeCell<&'static mut [DMADescriptor; 2]>,
    /// Endpoint 0 OUT buffers
    ///
    /// ## Invariants
    /// The `TakeCell` is never empty after a call to `init`.
    ep0_out_buffers: Cell<Option<&'static [[u8; 64]; 2]>>,
    /// Tracks the index in `ep0_out_descriptors` of the descriptor that will
    /// receive the next packet.
    ///
    /// ## Invariants
    /// Always less than the length of `ep0_out_descriptors` (2).
    next_out_idx: Cell<usize>,
    /// Tracks the index in `ep0_out_descriptors` of the descriptor that
    /// received the most recent packet.
    ///
    /// ## Invariants
    /// Always less than the length of `ep0_out_descriptors` (2).
    cur_out_idx: Cell<usize>,

    /// Endpoint 0 IN descriptors
    ///
    /// ## Invariants
    /// The `TakeCell` is never empty after a call to `init`.
    ep0_in_descriptors: TakeCell<&'static mut [DMADescriptor; 4]>,
    /// Endpoint 0 IN buffer
    ///
    /// `ep0_in_descriptors` point into the middle of this buffer but we copy
    /// into it as one big blob. This allows us to send large data (up to 256
    /// bytes) in one step by simply copying all the data into the buffer and
    /// configuring up to four descriptors.
    ///
    /// ## Invariants
    /// The `TakeCell` is never empty after a call to `init`.
    ep0_in_buffers: TakeCell<&'static mut [u8; 64 * 4]>,
}

/// Hardware base address of the singleton USB controller
const BASE_ADDR: *const Registers = 0x40300000 as *const Registers;

/// USB driver 0
pub static mut USB0: USB = unsafe { USB::new() };

/// OUT descriptors to pass into `USB#init`.
pub static mut OUT_DESCRIPTORS: [DMADescriptor; 2] =
    [DMADescriptor {
        flags: 0b11 << 30,
        addr: 0
    }; 2];
/// OUT buffers to pass into `USB#init`.
pub static mut OUT_BUFFERS: [[u8; 64]; 2] = [[0; 64]; 2];

/// IN descriptors to pass into `USB#init`.
pub static mut IN_DESCRIPTORS: [DMADescriptor; 4] =
    [DMADescriptor {
        flags: 0b11 << 30,
        addr: 0
    }; 4];
/// IN buffer to pass into `USB#init`.
pub static mut IN_BUFFERS: [u8; 64*4] = [0; 64 * 4];

impl USB {
    /// Creates a new value referencing the single USB driver.
    ///
    /// ## Safety
    ///
    /// Callers must ensure this is only called once for every program
    /// execution. Creating multiple instances will result in conflicting
    /// handling of events and can lead to undefined behavior.
    const unsafe fn new() -> USB {
        USB {
            registers: StaticRef::new(BASE_ADDR),
            core_clock: Clock::new(PeripheralClock::Bank1(
                    PeripheralClock1::Usb0)),
            timer_clock: Clock::new(PeripheralClock::Bank1(
                    PeripheralClock1::Usb0TimerHs)),
            state: Cell::new(USBState::WaitingForSetupPacket),
            ep0_out_descriptors: TakeCell::empty(),
            ep0_out_buffers: Cell::new(None),
            ep0_in_descriptors: TakeCell::empty(),
            ep0_in_buffers: TakeCell::empty(),
            next_out_idx: Cell::new(0),
            cur_out_idx: Cell::new(0),
        }
    }

    /// Sets up EP0 OUT to receive a setup packet from the host.
    ///
    /// The SETUP packet is less than 64 bytes, so we only need one descriptor.
    /// Sets the Last and Interrupt-on-completion bits and max size to 64 bytes.
    ///
    /// In preparation for a SETUP packet we only want to receive interrupts for
    /// OUT, not IN.
    fn expect_setup_packet(&self) {
        self.state.set(USBState::WaitingForSetupPacket);
        self.ep0_out_descriptors.map(|descs| {
            descs[self.next_out_idx.get()].flags = 1 << 27 | 1 << 25 | 64;
        });

        // EP0 OUT interrupts on
        self.registers.device_all_ep_interrupt_mask.set(
            self.registers.device_all_ep_interrupt_mask.get() | (1 << 16));
        // EP0 IN interrupts off
        self.registers.device_all_ep_interrupt_mask.set(
            self.registers.device_all_ep_interrupt_mask.get() & !1);

        // Enable OUT endpoint 0 and clear NAK bit
        self.registers.out_endpoints[0].control.set(1 << 31 | 1 << 26);
    }

    /// Call when a full packet has been received on EP0 OUT.
    /// Swaps the descriptors.
    fn got_rx_packet(&self) {
        self.ep0_out_descriptors.map(|descs| {
            let mut noi = self.next_out_idx.get();
            self.cur_out_idx.set(noi);
            noi = (noi + 1) % descs.len();
            self.next_out_idx.set(noi);
            self.registers.out_endpoints[0].dma_address.set(
                &descs[noi] as *const DMADescriptor as u32);
        });
    }

    /// Initialize descriptors for endpoint 0 IN and OUT
    ///
    /// Reset the endpoint 0 descriptors to a clean state and prepares for a
    /// SETUP packet from the host.
    fn init_descriptors(&self) {
        // Setup descriptor for OUT endpoint 0
        self.ep0_out_buffers.get().map(|bufs| {
            self.ep0_out_descriptors.map(|descs| {
                for (desc, buf) in descs.iter_mut().zip(bufs.iter()) {
                    desc.flags = 0b11 << 30;
                    desc.addr = buf.as_ptr() as usize;
                }
                self.next_out_idx.set(0);
                self.registers.out_endpoints[0].dma_address.set(
                    &descs[0] as *const DMADescriptor as u32);
            });
        });

        // Setup descriptor for IN endpoint 0
        self.ep0_in_buffers.map(|buf| {
            self.ep0_in_descriptors.map(|descs| {
                for (i, desc) in descs.iter_mut().enumerate() {
                    desc.flags = 0b11 << 30;
                    desc.addr = buf.as_ptr() as usize + i * 64;
                }
                self.registers.in_endpoints[0].dma_address.set(
                    &descs[0] as *const DMADescriptor as u32);
            });
        });


        self.expect_setup_packet();
    }

    /// Reset the device in response to a USB RESET
    fn reset(&self) {
        self.state.set(USBState::WaitingForSetupPacket);
        // Reset device address field (bits 10:4) of device config
        self.registers.device_config.set(
            self.registers.device_config.get() & !(0b1111111 << 4));

        self.init_descriptors();
    }

    /// Interrupt handler
    ///
    /// The Chip should call this from its `service_pending_interrupts` routine
    /// when an interrupt is received on the USB nvic line.
    ///
    /// Directly handles events related to device initialization, connection and
    /// disconnection, as well as control transfers on endpoint 0. Other events
    /// are passed to clients delegated for particular endpoints or interfaces.
    ///
    /// TODO(alevy): implement what this comment promises
    pub fn handle_interrupt(&self) {
        // Save current interrupt status snapshot so can clear only those at the
        // end
        let status = self.registers.interrupt_status.get();

        if status & USB_RESET != 0 {
            //println!("==> USB Reset");
            self.reset();
        }

        if status & ENUM_DONE != 0 {
            // MPS default set to 0 == 64 bytes
            //println!("==> ENUM DONE");
        }

        if status & EARLY_SUSPEND != 0 {
            println!("==> EARLY SUSPEND");
        }

        if status & USB_SUSPEND != 0 {
            println!("==> USB_SUSPEND");
        }

        if self.registers.interrupt_mask.get() & status & SOF != 0 {
            self.registers.interrupt_mask.set(
                self.registers.interrupt_mask.get() & !SOF);
        }

        if status & GOUTNAKEFF != 0 {
            self.registers.device_control.set(
                self.registers.device_control.get() |
                1 << 10); // Clear Global OUT NAK
        }

        if status & GINNAKEFF != 0 {
            self.registers.device_control.set(
                self.registers.device_control.get() |
                1 << 8); // Clear Global Non-periodic IN NAK
        }

        if status & (OEPINT | IEPINT) != 0 {
            //println!("==> OEPINT");

            let daint = self.registers.device_all_ep_interrupt.get();
            let inter_ep0_out = daint & 1 << 16 != 0;
            let inter_ep0_in = daint & 1 != 0;
            if inter_ep0_out || inter_ep0_in {
                self.handle_ep0(inter_ep0_out, inter_ep0_in);
            }
        }

        self.registers.interrupt_status.set(status);
    }

    /// Handle all endpoint 0 IN/OUT events
    fn handle_ep0(&self, inter_out: bool, inter_in: bool) {
        let ep_out = &self.registers.out_endpoints[0];
        let ep_out_interrupts = ep_out.interrupt.get();
        if inter_out {
            ep_out.interrupt.set(ep_out_interrupts);
        }

        let ep_in = &self.registers.in_endpoints[0];
        let ep_in_interrupts = ep_in.interrupt.get();
        if inter_in {
            ep_in.interrupt.set(ep_in_interrupts);
        }

        // Prepare next OUT descriptor if XferCompl
        if inter_out && ep_out_interrupts & 1 != 0 {
            self.got_rx_packet();
        }

        let transfer_type = TableCase::decode_interrupt(ep_out_interrupts);

        let flags = self.ep0_out_descriptors.map(|descs|
                            descs[self.cur_out_idx.get()].flags).unwrap();
        let setup_ready = flags & (1 << 24) != 0; // Setup Ready bit

        match self.state.get() {
            USBState::WaitingForSetupPacket => {
                if transfer_type == TableCase::A ||
                                transfer_type == TableCase::C {
                    if setup_ready {
                        self.handle_setup(transfer_type);
                    } else {
                        println!("Unhandled0 USB event {:#x} {:#x}", ep_out_interrupts, ep_in_interrupts);
                    }
                } else {
                    panic!("Very unexpected...");
                }
            },
            USBState::DataStageIn => {
                // TODO
                panic!("DataStageIn");
            },
            USBState::NoDataStage => {
                //TODO
                if inter_in  && ep_in_interrupts & 1 != 0 {
                    self.registers.in_endpoints[0].control.set(1 << 31);
                    println!("Input interrupt");
                }

                if inter_out {
                    if transfer_type == TableCase::B {
                        // IN detected
                        self.registers.in_endpoints[0].control.set(1 << 31 | 1 << 26);
                        self.registers.out_endpoints[0].control.set(1 << 31 | 1 << 26);
                    } else if transfer_type == TableCase::A ||
                                    transfer_type == TableCase::C {
                        if setup_ready {
                            self.handle_setup(transfer_type);
                        } else {
                            println!("Unhandled1 USB event {:#x} {:#x}", ep_out_interrupts, ep_in_interrupts);
                            self.expect_setup_packet();
                        }
                    } else {
                        println!("What's going on? {:?}", transfer_type);
                        self.expect_setup_packet();
                    }
                }
            }
        }
    }

    /// Handle SETUP packets to endpoint 0
    ///
    /// `transfer_type` is the `TableCase` found by inspecting endpoint-0's
    /// interrupt register.
    fn handle_setup(&self, transfer_type: TableCase) {
        // Assuming `ep0_out_buffers` was properly set in `init`, this will
        // always succeed.
        self.ep0_out_buffers.get().map(|bufs| {
            let buf = bufs[self.cur_out_idx.get()];

            let bm_request_type = buf[0];
            let b_request = buf[1];
            let w_value = buf[2] as u16 | ((buf[3] as u16) << 8);
            // wIndex
            let w_length = buf[6] as u16 | ((buf[7] as u16) << 8);

            let data_direction = (bm_request_type & 0x80) >> 7;
            let req_type = (bm_request_type & 0x60) >> 5;
            let recipient = bm_request_type & 0x1f;

            if req_type == 0 && recipient == 0 { // Standard device request
                if data_direction == 1 { // Device-to-host
                    // TODO
                    panic!("Device to host");
                } else if w_length > 0 { // Host-to-device
                    // TODO
                    panic!("Host to device");
                } else { // No data stage
                    match b_request {
                        5 /* Set Address */ => {
                            // Even though USB wants the address to be set after the
                            // IN packet handshake, the hardware knows to wait, so
                            // we should just set it now.
                            let dcfg = self.registers.device_config.get();
                            self.registers.device_config.set((dcfg & !(0x7f << 4)) |
                                (((w_value & 0x7f) as u32) << 4));
                            self.expect_status_phase_in(transfer_type);
                            println!("\tSetAddress: {}", w_value);
                        },
                        _ => {
                            panic!("Unhandled setup packet {}", b_request);
                        }
                    }
                }
            } else if recipient == 1 { // Interface
                // TODO
                panic!("Recipient is interface");
            }
        });
    }

    /// Setup endpoint 0 for a status phase with no data phase.
    fn expect_status_phase_in(&self, transfer_type: TableCase) {
        self.state.set(USBState::NoDataStage);

        self.ep0_in_descriptors.map(|descs| {
            // 1. Expect a zero-length in for the status phase
            // IOC, Last, Length 0, SP
            self.ep0_in_buffers.map(|buf| {
                descs[0].addr = buf.as_ptr() as usize; // Address doesn't matter since length is zero
            });
            descs[0].flags = 1 << 27 | 1 << 26 | 1 << 25 | 0;

            // 2. Flush fifos
            self.flush_tx_fifo(0);

            // 3. Set EP0 in DMA
            self.registers.in_endpoints[0].dma_address.set(
                &descs[0] as *const DMADescriptor as u32);
            //println!("{:#x}", self.registers.in_endpoints[0].dma_address.get());

            if transfer_type == TableCase::C && false {
                self.registers.in_endpoints[0].control.set(1 << 31 | 1 << 26);
            } else {
                self.registers.in_endpoints[0].control.set(1 << 31);
            }


            self.ep0_out_descriptors.map(|descs| {
                descs[self.next_out_idx.get()].flags = 1 << 27 | 1 << 25 | 64;
            });

            if transfer_type == TableCase::C && false {
                self.registers.out_endpoints[0].control.set(1 << 31 | 1 << 26);
            } else {
                self.registers.out_endpoints[0].control.set(1 << 31);
            }

            self.registers.device_all_ep_interrupt_mask.set(
                self.registers.device_all_ep_interrupt_mask.get() |
                1 | 1 << 16);
        });
    }

    /// Flush endpoint 0's RX FIFO
    ///
    /// # Safety
    ///
    /// Only call this when  transaction is not underway and data from this FIFO
    /// is not being copied.
    fn flush_rx_fifo(&self) {
        self.registers.reset.set(
            1 << 4); // TxFFlsh

        // Wait for TxFFlsh to clear
        while self.registers.reset.get() & 1 << 4 != 0 {}
    }

    /// Flush endpoint 0's TX FIFO
    ///
    /// `fifo_num` is 0x0-0xF for a particular fifo, or 0x10 for all fifos
    ///
    /// # Safety
    ///
    /// Only call this when  transaction is not underway and data from this FIFO
    /// is not being copied.
    fn flush_tx_fifo(&self, fifo_num: u8) {
        self.registers.reset.set(
            (fifo_num as u32) << 6 | // TxFIFO number: 0
            1 << 5);                 // TxFFlsh

        // Wait for TxFFlsh to clear
        while self.registers.reset.get() & 1 << 5 != 0 {}
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
        self.registers.transmit_fifo_size.set(
            ((TX_FIFO_SIZE as u32) << 16) |
            ((RX_FIFO_SIZE as u32) & 0xffff));
        for (i,d) in self.registers.device_in_ep_tx_fifo_size.iter().enumerate() {
            let i = i as u16;
            d.set(((TX_FIFO_SIZE as u32) << 16) |
                  (RX_FIFO_SIZE + i * TX_FIFO_SIZE) as u32);
        }

        self.flush_tx_fifo(0x10);
        self.flush_rx_fifo();

    }

    /// Perform a soft reset on the USB core. May timeout if the reset takes too
    /// long.
    fn soft_reset(&self) {
        self.registers.reset.set(1);

        let mut timeout = 10000;
        while self.registers.reset.get() & 1 == 1 && timeout > 0 {
            timeout -= 1;
        }
        if timeout == 0 {
            println!("USB: reset failed");
            return
        }

        let mut timeout = 10000;
        while self.registers.reset.get() & 1 << 31 == 0 && timeout > 0 {
            timeout -= 1;
        }
        if timeout == 0 {
            println!("USB: reset timeout");
            return
        }

    }

    /// Initialize the USB driver in device mode.
    ///
    /// Once complete, the driver will begin communicating with a connected
    /// host.
    pub fn init(&self,
                out_descriptors: &'static mut [DMADescriptor; 2],
                out_buffers: &'static mut [[u8; 64]; 2],
                in_descriptors: &'static mut [DMADescriptor; 4],
                in_buffers: &'static mut [u8; 64 * 4],
                phy: PHY) {
        self.ep0_out_descriptors.replace(out_descriptors);
        self.ep0_out_buffers.set(Some(out_buffers));
        self.ep0_in_descriptors.replace(in_descriptors);
        self.ep0_in_buffers.replace(in_buffers);

        // ** GLOBALSEC **
        // TODO(alevy): refactor out
        unsafe {
            use core::intrinsics::volatile_store as vs;

            vs(0x40090000 as *mut u32, !0);
            vs(0x40090004 as *mut u32, !0);
            vs(0x40090008 as *mut u32, !0);
            vs(0x4009000c as *mut u32, !0);

            // GLOBALSEC_DDMA0-DDMA3
            vs(0x40090080 as *mut u32, !0);
            vs(0x40090084 as *mut u32, !0);
            vs(0x40090088 as *mut u32, !0);
            vs(0x4009008c as *mut u32, !0);

            // GLOBALSEC_DUSB_REGION0-DUSB_REGION3
            vs(0x400900c0 as *mut u32, !0);
            vs(0x400900c4 as *mut u32, !0);
            vs(0x400900c8 as *mut u32, !0);
            vs(0x400900cc as *mut u32, !0);
        }

        self.core_clock.enable();
        self.timer_clock.enable();

        self.registers.interrupt_mask.set(0);
        self.registers.device_all_ep_interrupt_mask.set(0);
        self.registers.device_in_ep_interrupt_mask.set(0);
        self.registers.device_out_ep_interrupt_mask.set(0);

        let sel_phy = match phy {
            PHY::A => 0b100,
            PHY::B => 0b101,
        };
        // Select PHY A
        self.registers.gpio.set((1 << 15 | // WRITE mode
                                sel_phy << 4 | // Select PHY A & Set PHY active
                                0) << 16); // CUSTOM_CFG Register

        // Configure the chip
        self.registers.configuration.set(
            1 << 6 | // USB 1.1 Full Speed
            0 << 5 | // 6-pin unidirectional
            14 << 10 | // USB Turnaround time to 14 -- what does this mean though??
            7); // Timeout calibration to 7 -- what does this mean though??


        // Soft reset
        self.soft_reset();

        // Configure the chip
        self.registers.configuration.set(
            1 << 6 | // USB 1.1 Full Speed
            0 << 5 | // 6-pin unidirectional
            14 << 10 | // USB Turnaround time to 14 -- what does this mean though??
            7); // Timeout calibration to 7 -- what does this mean though??

        //=== Begin Core Initialization ==//

        // We should be reading `user_hw_config` registers to find out about the
        // hardware configuration (which endpoints are in/out, OTG capable,
        // etc). Skip that for now and just make whatever assumption CR50 is
        // making.

        // Set the following parameters:
        //   * Enable DMA Mode
        //   * Global unmask interrupts
        //   * Interrupt on Non-Periodic TxFIFO completely empty
        // _Don't_ set:
        //   * Periodic TxFIFO interrupt on empty (only valid in slave mode)
        //   * AHB Burst length (defaults to 1 word)
        self.registers.ahb_config.set(1 |      // Global Interrupt unmask
                                      1 << 5 | // DMA Enable
                                      1 << 7); // Non_periodic TxFIFO

        // Set Soft Disconnect bit to make sure we're in disconnected state
        self.registers.device_control.set(
            self.registers.device_control.get() | (1 << 1));

        // The datasheet says to unmask OTG and Mode Mismatch interrupts, but
        // we don't support anything but device mode for now, so let's skip
        // handling that
        //
        // If we're right, then
        // `self.registers.interrupt_status.get() & 1 == 0`
        //

        //=== Done with core initialization ==//

        //===  Begin Device Initialization  ==//

        self.registers.device_config.set(
            self.registers.device_config.get() |
            0b11       | // Device Speed: USB 1.1 Full speed (48Mhz)
            0 << 2     | // Non-zero-length Status: send packet to application
            0b00 << 11 | // Periodic frame interval: 80%
            1 << 23 );   // Enable Scatter/gather

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
        self.registers.device_out_ep_interrupt_mask.set(
            1 << 0 | // XferCompl
            1 << 1 | // Disabled
            1 << 3); // SETUP
        //    Device IN XferCompl & TimeOut
        self.registers.device_in_ep_interrupt_mask.set(
            1 << 0 | // XferCompl
            1 << 1); // Disabled

        // To set ourselves up for processing the state machine through interrupts,
        // unmask:
        //
        //   * USB Reset
        //   * Enumeration Done
        //   * Early Suspend
        //   * USB Suspend
        //   * SOF
        //
        self.registers.interrupt_mask.set(
            GOUTNAKEFF | GINNAKEFF |
            USB_RESET | ENUM_DONE |
            OEPINT | IEPINT |
            EARLY_SUSPEND | USB_SUSPEND |
            SOF);

        // Power on programming done
        self.registers.device_control.set(
            self.registers.device_control.get() | 1 << 11);
        for _ in 0..10000 {
            ::support::nop();
        }
        self.registers.device_control.set(
            self.registers.device_control.get() & !(1 << 11));

        // Clear global NAKs
        self.registers.device_control.set(
            self.registers.device_control.get() |
            1 << 10 | // Clear global OUT NAK
            1 << 8);  // Clear Global Non-periodic IN NAK

        // Reconnect:
        //  Clear the Soft Disconnect bit to allow the core to issue a connect.
        self.registers.device_control.set(
            self.registers.device_control.get() & !(1 << 1));

    }
}

/// Which physical connection to use
pub enum PHY {
    A, B
}

/// Combinations of OUT endpoint interrupts for control transfers
///
/// Encodes the cases in from Table 10.7 in the Programming Guide (pages
/// 279-230).
#[derive(Copy,Clone,PartialEq,Eq,Debug)]
pub enum TableCase {
    /// Case A
    ///
    /// * StsPhseRcvd: 0
    /// * SetUp: 0
    /// * XferCompl: 1
    A,
    /// Case B
    ///
    /// * StsPhseRcvd: 0
    /// * SetUp: 1
    /// * XferCompl: 0
    B,
    /// Case C
    ///
    /// * StsPhseRcvd: 0
    /// * SetUp: 1
    /// * XferCompl: 1
    C,
    /// Case D
    ///
    /// * StsPhseRcvd: 1
    /// * SetUp: 0
    /// * XferCompl: 0
    D,
    /// Case E
    ///
    /// * StsPhseRcvd: 1
    /// * SetUp: 0
    /// * XferCompl: 1
    E
}

impl TableCase {
    /// Decodes a value from the OUT endpoint interrupt register.
    ///
    /// Only properly decodes values with the combinations shown in the
    /// programming guide.
    pub fn decode_interrupt(device_out_int: u32) -> TableCase {
        if device_out_int & (1 << 0) != 0 { // XferCompl
            if device_out_int & (1 << 3) != 0 { // Setup
                TableCase::C
            } else if device_out_int & (1 << 5) != 0 { // StsPhseRcvd
                TableCase::E
            } else {
                TableCase::A
            }
        } else {
            if device_out_int & (1 << 3) != 0 { // Setup
                TableCase::B
            } else {
                TableCase::D
            }
        }
    }
}

interrupt_handler!(usb_handler, 193);
