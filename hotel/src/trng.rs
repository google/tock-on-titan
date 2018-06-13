//! Driver for the True Random Number Generator (TRNG).

use core::cell::Cell;
use kernel::hil::rng::{Continue, RNG, Client};
use kernel::common::volatile_cell::VolatileCell;

#[allow(dead_code)]
#[repr(C)]
struct Registers {
    /// TRNG version.  Defaults to 0x2d013316.
    _version: VolatileCell<u32>,

    /// Enable interrupts.
    interrupt_enable: VolatileCell<u32>,

    /// Current state of interrupts.
    interrupt_state: VolatileCell<u32>,

    /// Used to test interrupts.
    interrupt_test: VolatileCell<u32>,

    /// High permission register to control which post-processing techniques
    /// should be turned on.
    secure_post_processing_control: VolatileCell<u32>,

    /// Medium permission register to control post-processing techniques.
    post_processing_control: VolatileCell<u32>,

    /// Single pulse from the processor to initiate the digital TRNG by moving the
    /// FSM from the idle to the active calibrated stage.
    go_event: VolatileCell<u32>,

    /// Counter for the timeout that determines how long the digital TRNG will wait
    /// for its analog counterpart.
    timeout_counter: VolatileCell<u32>,

    /// Maximum number of times the digital segment can timeout before interrupting
    /// the processor.
    timeout_max_try_num: VolatileCell<u32>,

    /// Random output refresh counter.  Every N cycles, the TRNG sends the random
    /// bits output to local modules, stopping its current state.
    output_time_counter: VolatileCell<u32>,

    /// Single pulse from the processor to shut down the TRNG by resetting the digital
    /// side to the idle state and save power.
    stop_work: VolatileCell<u32>,

    /// Debug register for keeping track of the FSM state.
    fsm_state: VolatileCell<u32>,

    /// Used to control the programmable minimum and maximum value that can be
    /// produced by the analog component.
    allowed_values: VolatileCell<u32>,

    /// Reflects the current time counter value that the TRNG has while waiting for
    /// its analog counterpart.
    timer_counter: VolatileCell<u32>,

    /// Most significant bits for the slicing portion that are used during calibration.
    slice_max_upper_limit: VolatileCell<u32>,

    /// Least significant bits for the slicing portion that are used during calibration.
    slice_min_lower_limit: VolatileCell<u32>,

    /// Maximum value seen during calibration.
    max_value: VolatileCell<u32>,

    /// Minimum value seen during calibration.
    min_value: VolatileCell<u32>,

    /// Reflects the current LDO settings from the processor and sends it to the analog TRNG.
    ldo_ctrl: VolatileCell<u32>,

    /// Powers down all the components of the TRNG when asserted low.
    power_down_b: VolatileCell<u32>,

    /// High permission register to disable power down control by application.
    proc_lock_power_down_b: VolatileCell<u32>,

    /// Analog test control.
    antest: VolatileCell<u32>,

    /// Input controls to laser detector unit.
    analog_sen_lsr_input: VolatileCell<u32>,

    /// Output controls to the laser detector unit.
    analog_sen_lsr_output: VolatileCell<u32>,

    /// Guides controls during device state testing.
    analog_test: VolatileCell<u32>,

    /// Control register to pass values to different analog controls.
    analog_ctrl: VolatileCell<u32>,

    /// Enables the TRNG one shot mode.
    one_shot_mode: VolatileCell<u32>,

    /// Stores the 16-bit output from the analog unit when one shot mode is enabled.
    one_shot_register: VolatileCell<u32>,

    /// TRNG output.
    read_data: VolatileCell<u32>,

    /// Indicates the number of times the processor asserts the TRNG_READ signal to get
    /// a fresh set of random 32 bits.  Cumulative until cleared by the processor.
    frequency_calls: VolatileCell<u32>,

    /// Indicates the number of bits that have the value of 1 among the total bits read
    /// by the digital part of the TRNG.
    cur_num_ones: VolatileCell<u32>,

    /// Indicates that the TRNG is currently empty.
    empty: VolatileCell<u32>,
}

const TRNG0_BASE: *mut Registers = 0x40410000 as *mut Registers;

pub static mut TRNG0: Trng<'static> = unsafe { Trng::new(TRNG0_BASE) };

pub struct Trng<'a> {
    regs: *mut Registers,
    client: Cell<Option<&'a Client>>,
}

impl<'a> Trng<'a> {
    const unsafe fn new(trng: *mut Registers) -> Trng<'a> {
        Trng {
            regs: trng,
            client: Cell::new(None),
        }
    }

    pub fn handle_interrupt(&self) {
        let regs = unsafe { &*self.regs };

        // Disable and clear the interrupt.
        regs.interrupt_enable.set(0);
        regs.interrupt_state.set(0x1);

        self.client.get().map(|client| {
            if let Continue::More = client.randomness_available(&mut Iter(self)) {
                // Re-enable the interrupt since the client needs more data.
                regs.interrupt_enable.set(0x1);
            }
        });
    }
}

impl<'a> RNG<'a> for Trng<'a> {
    
    fn set_client(&self, client: &'a Client) {
        self.client.set(Some(client));
    }

    fn init(&self) {
        let regs = unsafe { &*self.regs };

        // Enable bit shuffling and churn mode.  Disable XOR and Von Neumann processing.
        regs.post_processing_control.set(0xa);
        regs.slice_max_upper_limit.set(1);
        regs.slice_min_lower_limit.set(0);
        regs.timeout_counter.set(0x7ff);
        regs.timeout_max_try_num.set(4);
        regs.power_down_b.set(1);
        regs.go_event.set(1);
    }


    
    fn get(&self) {
        let regs = unsafe { &*self.regs };

        if regs.empty.get() > 0 {
            // Make sure the TRNG isn't stuck.
            if regs.fsm_state.get() & 0x8 != 0 {
                // TRNG timed out.  Restart.
                regs.stop_work.set(1);
                regs.go_event.set(1);
            }

            // Enable interrupts so we know when there is random data ready.
            regs.interrupt_enable.set(0x1);
        } else {
            self.client.get().map(|client| {
                if let Continue::More = client.randomness_available(&mut Iter(self)) {
                    regs.interrupt_enable.set(0x1);
                }
            });
        }
    }
}

struct Iter<'a, 'b: 'a>(&'a Trng<'b>);

impl<'a, 'b> Iterator for Iter<'a, 'b> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        let regs = unsafe { &*self.0.regs };

        if regs.empty.get() == 0 {
            Some(regs.read_data.get())
        } else {
            None
        }
    }
}
