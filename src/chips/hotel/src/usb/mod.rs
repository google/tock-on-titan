use pmu::{Clock, PeripheralClock, PeripheralClock1};
use core::cell::Cell;

mod constants;
mod registers;

use self::constants::*;
use self::registers::Registers;

struct StaticRef<T> {
    ptr: *const T
}

impl<T> StaticRef<T> {
    pub const unsafe fn new(ptr: *const T) -> StaticRef<T> {
        StaticRef { ptr: ptr }
    }
}

#[derive(Clone,Copy,PartialEq,Eq)]
enum USBState {
	WaitingForSetupPacket,
	DataStageIn,
	NoDataStage,
}

pub struct USB {
    registers: StaticRef<Registers>,
    core_clock: Clock,
    timer_clock: Clock,
    state: Cell<USBState>
}

const BASE_ADDR: *const Registers = 0x40300000 as *const Registers;

pub static mut USB0: USB = unsafe { USB::new() };

impl<T> ::core::ops::Deref for StaticRef<T> {
    type Target = T;
    fn deref(&self) -> &'static T {
        unsafe { &*self.ptr }
    }

}

static mut BUF: [[u8; 64]; 2] = [[0; 64]; 2];
static mut DESC: [registers::DMADescriptor; 2] =
    [registers::DMADescriptor {
        flags: 0b11 << 30,
        addr: 0
    }; 2];

impl USB {

    pub const unsafe fn new() -> USB {
        USB {
            registers: StaticRef::new(BASE_ADDR),
            core_clock: Clock::new(PeripheralClock::Bank1(
                    PeripheralClock1::Usb0)),
            timer_clock: Clock::new(PeripheralClock::Bank1(
                    PeripheralClock1::Usb0TimerHs)),
            state: Cell::new(USBState::WaitingForSetupPacket)
        }
    }

    pub fn handle_interrupt(&self) {
        // Save current interrupt status snapshot so can clear only those at the
        // end
        let status = self.registers.interrupt_status.get();

        if status & USB_RESET != 0 {
            println!("==> USB RESET");
            self.state.set(USBState::WaitingForSetupPacket);
            // 1. Set the nak bit for all OUT endpoitns
            // TODO(alevy): do we actually need to do this for _all_ endpoints?
            //              or just those we intend to enable?
            for ctrl in self.registers.out_endpoints.iter().map(|e|&e.control) {
                ctrl.set(1 << 27);
            }

            // 2. Unmask control 0 IN/OUT
            self.registers.device_all_ep_interrupt_mask.set(
                self.registers.device_all_ep_interrupt_mask.get() |
                1 << 0  | // IN 0
                1 << 16); // OUT 0
            //    Device OUT SETUP & XferCompl
            self.registers.device_out_ep_interrupt_mask.set(
                self.registers.device_out_ep_interrupt_mask.get() |
                1 << 0 | // XferCompl
                1 << 3); // SETUP
            //    Device IN XferCompl & TimeOut
            self.registers.device_in_ep_interrupt_mask.set(
                self.registers.device_in_ep_interrupt_mask.get() |
                1 << 0 | // XferCompl
                1 << 3); // TimeOout

            // 3. Set up data FIFO RAM
            self.registers.receive_fifo_size.set(RX_FIFO_SIZE as u32 & 0xffff);
            self.registers.transmit_fifo_size.set(
                (TX_FIFO_SIZE as u32) << 16 |
                (RX_FIFO_SIZE as u32) & 0xffff);
            for (i,d) in self.registers.device_in_ep_tx_fifo_size.iter().enumerate() {
                let i = i as u16;
                d.set((TX_FIFO_SIZE as u32) << 16 |
                      (RX_FIFO_SIZE + i * TX_FIFO_SIZE) as u32);
            }

            // TODO(alevy): Flush TxFIFOs and RxFIFO

            // 4. Reset device address field (bits 10:4) of device config
            self.registers.device_config.set(
                self.registers.device_config.get() & !(0b1111111 << 4));
        }

        if status & ENUM_DONE != 0 {
            println!("==> ENUM DONE");
            // MPS default set to 0 == 64 bytes

            // Setup descriptor for OUT endpoint 0
            unsafe {
                DESC[0].flags = 1 << 27 | 1 << 25 | 64;
                DESC[0].addr = BUF[0].as_ptr() as usize;
                self.registers.out_endpoints[0].dma_address.set(
                    &DESC[0] as *const registers::DMADescriptor as u32);
            }
            // Enable OUT endpoint 0 and clear NAK bit
            self.registers.out_endpoints[0].control.set(1 << 31 | 1 << 26);
        }

        if status & EARLY_SUSPEND != 0 {
            println!("==> EARLY SUSPEND");
        }

        if status & USB_SUSPEND != 0 {
            println!("==> USB_SUSPEND");
        }

        if self.registers.interrupt_mask.get() & status & SOF != 0 {
            println!("==> SOF");
            self.registers.interrupt_mask.set(
                self.registers.interrupt_mask.get() & !SOF);
        }

        if status & OEPINT != 0 {
            println!("==> OEPINT");

            let daint = self.registers.device_all_ep_interrupt.get();
            if daint & 1 << 16 != 0 {
                self.handle_ep0_out();
            } else {
                panic!("Unexpected!");
            }
        }

        self.registers.interrupt_status.set(status);
    }

    fn handle_ep0_out(&self) {
        print!("EP0: ");
        let ep0 = &self.registers.out_endpoints[0];
        let ep0_interrupts = ep0.interrupt.get();
        ep0.interrupt.set(ep0_interrupts);

        let transfer_type = decode_table_10_7(ep0_interrupts);

        let flags = unsafe { ::core::intrinsics::volatile_load(&DESC[0].flags) };
        let setup_ready = flags & (1 << 24) != 0; // Setup Ready bit

        match self.state.get() {
            USBState::WaitingForSetupPacket => {
                if transfer_type == TableCase::A ||
                                    transfer_type == TableCase::C {
                    if setup_ready {
                        println!("Setup packet read");
                        self.handle_setup(transfer_type);
                    } else {
                        panic!("Unhandled USB event {:#x}", ep0_interrupts);
                    }
                }
            },
            USBState::DataStageIn => {
                // TODO
            },
            USBState::NoDataStage => {
                //TODO
            }
        }
    }

    fn handle_setup(&self, transfer_type: TableCase) {
        let buf = unsafe { BUF[0] };

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
            } else if w_length > 0 { // Host-to-device
                // TODO
            } else { // No data stage
                match b_request {
                    5 /* Set Address */ => {
                        // Even though USB wants the address to be set after the
                        // IN packet handshake, the hardware knows to wait, so
                        // we should just set it now.
                        println!("\tSetAddress: {}", w_value);
                        let dcfg = self.registers.device_config.get();
                        self.registers.device_config.set((dcfg & !(0x7f << 4)) |
                            (((w_value & 0x7f) as u32) << 4));
                    },
                    9 /* Set configuration */ => {
                        // TODO
                    },
                    7 /* Set descriptor */ => {
                        // TODO
                    },
                    3 /* Set feature */ => {
                        // TODO
                    }
                    _ => {}
                }
                self.expect_status_phase_in(transfer_type);
            }
        } else if recipient == 1 { // Interface
            // TODO
        }
    }

    fn expect_status_phase_in(&self, transfer_type: TableCase) {
        self.state.set(USBState::NoDataStage);

        // 1. Expect a zero-length in for the status phase
        // 2. Flush fifos
        // 3. Set EP0 in DMA
        // etc...
    }

    pub fn init(&self) {

        // ** GLOBALSEC **
        // TODO(alevy): refactor out
        unsafe {
            use core::intrinsics::volatile_store as vs;
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

        // Select PHY A
        self.registers.gpio.set((1 << 15 | // WRITE mode
                                0b100 << 4 | // Select PHY A & Set PHY active
                                0) << 16); // CUSTOM_CFG Register

        self.registers.reset.set(1);
        while self.registers.reset.get() & 1 == 1 {}

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

        // Mask all specific interrupts for now
        self.registers.interrupt_mask.set(0);
        self.registers.device_in_ep_interrupt_mask.set(0);
        self.registers.device_out_ep_interrupt_mask.set(0);
        self.registers.device_all_ep_interrupt_mask.set(0);

        // Configure the chip
        self.registers.configuration.set(
            1 << 6 | // USB 1.1 Full Speed
            0 << 5 | // 6-pin unidirectional
            14 << 10 | // USB Turnaround time to 14 -- what does this mean though??
            7); // Timeout calibration to 7 -- what does this mean though??

        // The datasheet says to unmask OTG and Mode Mismatch interrupts, but
        // we don't support anything but device mode for now, so let's skip
        // handling that
        //
        // If we're right, then
        // `self.registers.interrupt_status.get() & 1 == 0`
        //

        //=== Done with core initialization ==//

        //===  Begin Device Initialization  ==//

        // Clear the Soft Disconnect bit to allow the core to issue a connect.
        self.registers.device_control.set(
            self.registers.device_control.get() | 1 << 1);


        self.registers.device_config.set(
            0b11       | // Device Speed: USB 1.1 Full speed (48Mhz)
            0 << 2     | // Non-zero-length Status: semd packet to application
            0b00 << 11 | // Periodic frame interval: 80%
            1 << 23 );   // Enable Scatter/gather

        // We would set the device threshold control register here, but I don't
        // think we enable thresholding.

        // Clear the Soft Disconnect bit to allow the core to issue a connect.
        self.registers.device_control.set(
            self.registers.device_control.get() & !(1 << 1));

        // To set ourselves for processing the state machine through interrupts,
        // unmask:
        //
        //   * USB Reset
        //   * Enumeration Done
        //   * Early Suspend
        //   * USB Suspend
        //   * SOF
        //
        self.registers.interrupt_mask.set(
            SOF | EARLY_SUSPEND | USB_SUSPEND | USB_RESET | ENUM_DONE | OEPINT);
    }
}

#[derive(Copy,Clone,PartialEq,Eq)]
enum TableCase {
    A, B, C, D, E
}

fn decode_table_10_7(device_out_int: u32) -> TableCase {
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

interrupt_handler!(usb_handler, 193);
