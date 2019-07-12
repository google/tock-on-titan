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

//! Power Management Unit (PMU)
//!
//! The Power Management Unit (PMU) controls the chip's power states and clock
//! power, as well as allows the chip to interface with peripherals at different
//! voltage levels.
//!
//! There are three types of power domains:
//!
//!   1. VDDIOMR:
//!
//!     * Always-on main I/O voltage supply domain necessary for the running the
//!       core.
//!     * Nominal 3.3V
//!
//!   2. VDDIOA & VDDIOB:
//!
//!     * I/O voltage supply domains for external peripherals.
//!
//!     * Can be shut down at any moment (not critical for core functions).
//!
//!     * Designed for 1.8-3.6V
//!
//!   3. VDDIOF:
//!
//!     * I/O voltage supply domain for flash memory.
//!
//!     * Designed for 1.8-3.6V
//!

use cortexm3;

use kernel::common::cells::VolatileCell;
use kernel::common::registers::{ReadOnly, ReadWrite};
use kernel::common::StaticRef;

use usb::u2f::UsbHidU2f;

register_bitfields![u32,
    Reset [
        Value           OFFSET(0) NUMBITS(32) []
    ],
    ClearReset [
        Value           OFFSET(0) NUMBITS(32) []
    ],
    ResetSource [
        PowerOnReset              0,
        LowPowerExit              1,
        Watchdog                  2,
        Lockup                    3,
        SysReset                  4,
        Software                  5,
        Brownout                  6,
        Security                  7
    ],
    GlobalReset [
        Value           OFFSET(0) NUMBITS(32) []
    ],
    LowPowerDisable [
        Start                     0,
        VddlOff                   1,
        FlashOff                  2,
        OscillatorOff             3,
        JitterRCOff               4
    ],
    LowPowerBypass [
        Vddl                      0,
        Flash                     1,
        Oscillator                2,
        OscillatorComparator      3,
        JitterRC                  4,
        TimerRC                   5,
        FlashPowerDown            6,
        VddlVddaonIsolation       7,
        VddxoVddaonIsolation      8
    ],
    WakeupInterruptController [
        Processor0                0
    ],
    SystemVtor [
        Value           OFFSET(0) NUMBITS(32) []
    ],
    NapEnable [
        Value           OFFSET(0) NUMBITS(32) []
    ],
    BatteryLevelOk [
        OK                        0
    ],
    ExitPowerdown [
        EnablePinPd               0,
        EnableUtmiSuspend         1,
        EnableRdd0PdTimer         2,
        EnableTimeLs0Timer0       3,
        EnableTimeLs0Timer1       4,
        EnableRboxWakeup          5
    ],
    MemoryClock [
        BankClock0                0,
        BankClock1                1,
        BankClock2                2,
        BankClock3                3,
        BankClock4                4,
        BankClock5                5,
        BankClock6                6
    ],
    PeripheralClock0 [
        Camo                      0,
        Dcrypto                   1,
        Dma                       2,
        Flash                     3,
        Fuse                      4,
        GlobalSec                 5,
        GlobalSecTimer            6,
        GlobalSecHs               7,
        Gpio0                     8,
        Gpio1                     9,
        I2C0                     10,
        I2C1                     11,
        I2CS0                    12,
        KeyManager               13,
        PeripheralApb0           14,
        PeripheralApb1           15,
        PeripheralApb2           16,
        PeripheralApb2Timer      17,
        PeripheralApb3           18,
        PeripheralApb3Hs         19,
        PinMux                   20,
        Pmu                      21,
        RBox                     22,
        Rdd                      23,
        Rtc                      24,
        RtcTimer                 25,
        Spi0Hs                   26,
        Spi1Hs                   27,
        Sps                      28,
        SpsTimerHs               29,
        Swdp                     30,
        Temp                     31
    ],
    PeripheralClock1 [
        TimeHs0Timer              0,
        TimeHs1Timer              1,
        TimeLs                    2,
        Timerus                   3,
        Trng                      4,
        Uart0                     5,
        Uart1                     6,
        Uart2                     7,
        Usb                       8,
        UsbTimerHs                9,
        VoltRO                   10,
        WatchdogRO               11,
        XO                       12,
        XOTimer                  13,
        PeripheralMasterMatrix   14,
        PeripheralMaster         15
    ]
];

/// Registers for the Power Management Unit (PMU)
// Non-public fields prefixed with "_" mark unused registers
#[repr(C)]
pub struct Registers {
    reset:                                 ReadWrite<u32, Reset::Register>,
    _set_reset: VolatileCell<u32>,
    pub clear_reset:                       ReadWrite<u32, ClearReset::Register>,
    pub reset_source:                      ReadOnly<u32, ResetSource::Register>,

    /// To initiate a reset, write the key 0x7041776 to global_reset.
    pub global_reset:                      ReadWrite<u32, GlobalReset::Register>,
    pub low_power_disable:                 ReadWrite<u32, LowPowerDisable::Register>,
    pub low_power_bypass:                  ReadWrite<u32, LowPowerBypass::Register>,
    pub low_power_bypass_value:            ReadWrite<u32, LowPowerBypass::Register>,
    pub set_wakeup_interrupt_controller:   ReadWrite<u32, WakeupInterruptController::Register>,
    pub clear_wakeup_interrupt_controller: ReadWrite<u32, WakeupInterruptController::Register>,
    /// Value of the system vector table offset
    pub sysvtor:                           ReadWrite<u32, SystemVtor::Register>,
    pub nap_enable:                        ReadWrite<u32, NapEnable::Register>,

    _pmu_sw_pdb: VolatileCell<u32>,
    _pmu_sw_pdb_secure: VolatileCell<u32>,
    _pmu_vref: VolatileCell<u32>,
    _xtl_osc_bypass: VolatileCell<u32>,
    _flash_tm0_test_en_bypass: VolatileCell<u32>,
    pub battery_level_ok:                  ReadOnly<u32, BatteryLevelOk::Register>,

    _b_reg_dig_ctrl: VolatileCell<u32>,
    exitpd_mask:                           ReadWrite<u32, ExitPowerdown::Register>,
    exitpd_src:                            ReadWrite<u32, ExitPowerdown::Register>,
    exitpd_mon:                            ReadWrite<u32, ExitPowerdown::Register>,
    _osc_ctrl: VolatileCell<u32>,

    pub memory_clock_set:                  ReadWrite<u32, MemoryClock::Register>,
    pub memory_clock_clear:                ReadWrite<u32, MemoryClock::Register>,

    pub peripheral_clocks0_enable:         ReadWrite<u32, PeripheralClock0::Register>,
    pub peripheral_clocks0_disable:        ReadWrite<u32, PeripheralClock0::Register>,
    pub peripheral_clocks1_enable:         ReadWrite<u32, PeripheralClock1::Register>,
    pub peripheral_clocks1_disable:        ReadWrite<u32, PeripheralClock1::Register>,

    pub _peripheral_clocks0_ro_mask: VolatileCell<u32>,
    pub _peripheral_clocks1_ro_mask: VolatileCell<u32>,

    pub _gate_on_sleep_set0: VolatileCell<u32>,
    pub _gate_on_sleep_clr0: VolatileCell<u32>,

    pub _gate_on_sleep_set1: VolatileCell<u32>,
    pub _gate_on_sleep_clr1: VolatileCell<u32>,

    pub _clock0: VolatileCell<u32>,
    pub _reset0_write_enable: VolatileCell<u32>,
    pub reset0:                            ReadWrite<u32, PeripheralClock0::Register>,

    pub _reset1_write_enable: VolatileCell<u32>,
    pub _reset1:                           ReadWrite<u32, PeripheralClock1::Register>

}

const PMU_REGISTERS: StaticRef<Registers> =
    unsafe { StaticRef::new(0x40000000 as *const Registers) };


#[derive(Clone,Copy)]
pub enum Peripheral0 {
    Camo0,
    Crypto0,
    Dma0,
    Flash0,
    Fuse0,
    GlobalSec,
    GlobalSecTimer,
    GlobalSecHs,
    Gpio0,
    Gpio1,
    I2C0,
    I2C1,
    I2CS0,
    KeyMgr0,
    PeriAPB0,
    PeriAPB1,
    PeriAPB2,
    PeriAPB2Timer,
    PeriAPB3,
    PeriAPB3Timer,
    PeriAPB3HS,
    PinMux,
    Pmu,
    RBox0,
    Rdd0,
    Rtc0,
    Rtc0Timer,
    Spi0Hs,
    Spi1Hs,
    Sps0,
    Sps0TimerHs,
    Swdp0,
    Temp0,
}

#[derive(Clone,Copy)]
pub enum Peripheral1 {
    TimeHs0Timer,
    TimeHs1Timer,
    TimeLs0,
    TimeUs0Timer,
    Trng0,
    Uart0Timer,
    Uart1Timer,
    Uart2Timer,
    Usb0,
    Usb0TimerHs,
    Volt0,
    Watchdog0,
    Xo0,
    Xo0Timer,
    PeripheralMasterMatrix,
    PeripheralMatrix,
}

#[derive(Clone,Copy)]
pub enum PeripheralClock {
    Bank0(Peripheral0),
    Bank1(Peripheral1),
}

/// Wrapper struct around `PeripheralClock` that can only be created by.
/// trusted code.
#[derive(Clone,Copy)]
pub struct Clock {
    // It's important that this field is private!
    clock: PeripheralClock,
}

impl Clock {
    pub const unsafe fn new(clock: PeripheralClock) -> Clock {
        Clock { clock: clock }
    }

    pub fn enable(&self) {
        let registers = &*PMU_REGISTERS;
        match self.clock {
            PeripheralClock::Bank0(clock) => {
                registers.peripheral_clocks0_enable.set(1 << (clock as u32));
            }
            PeripheralClock::Bank1(clock) => {
                registers.peripheral_clocks1_enable.set(1 << (clock as u32));
            }
        }
    }

    pub fn disable(&self) {
        let registers = &*PMU_REGISTERS;
        match self.clock {
            PeripheralClock::Bank0(clock) => {
                registers.peripheral_clocks0_disable.set(1 << (clock as u32));
            }
            PeripheralClock::Bank1(clock) => {
                registers.peripheral_clocks1_disable.set(1 << (clock as u32));
            }
        }
    }
}
// This should be refactored to be a general reset
pub fn reset_dcrypto() {
    let registers = &*PMU_REGISTERS;
    registers.reset0.modify(PeripheralClock0::Dcrypto::CLEAR);
}


static mut SLEEP_DEEPLY: bool = false;

pub fn enable_deep_sleep() {
    //debug!("PMU: Enabling DEEP sleep.\n");
    unsafe {SLEEP_DEEPLY = true;}
}

pub fn disable_deep_sleep() {
    //debug!("PMU: Disabling DEEP sleep.\n");
    unsafe {SLEEP_DEEPLY = false;}
}

pub fn prepare_for_sleep() {
    let registers = &*PMU_REGISTERS;
/*
        unsafe {
        static mut val: usize = 0;
        val = val + 1;

        if (val % 20 == 0) {
            debug!("Prepare_For_Sleep: {}", val);
        }
    }
     */
    /*    GR_PMU_EXITPD_MASK =
                GC_PMU_EXITPD_MASK_PIN_PD_EXIT_MASK |
                GC_PMU_EXITPD_MASK_RDD0_PD_EXIT_TIMER_MASK |
                GC_PMU_EXITPD_MASK_RBOX_WAKEUP_MASK |
                GC_PMU_EXITPD_MASK_TIMELS0_PD_EXIT_TIMER0_MASK |
                GC_PMU_EXITPD_MASK_TIMELS0_PD_EXIT_TIMER1_MASK;

    plus

        if (utmi_wakeup_is_enabled() && idle_action != IDLE_DEEP_SLEEP)
                GR_PMU_EXITPD_MASK |=
                        GC_PMU_EXITPD_MASK_UTMI_SUSPEND_N_MASK;
    */

    registers.exitpd_mask.write(ExitPowerdown::EnablePinPd::SET +
                                ExitPowerdown::EnableRdd0PdTimer::SET +
                                ExitPowerdown::EnableRboxWakeup::SET +
                                ExitPowerdown::EnableTimeLs0Timer0::SET +
                                ExitPowerdown::EnableTimeLs0Timer1::SET);
    let deep = unsafe {SLEEP_DEEPLY};

    if !deep {
        registers.exitpd_mask.modify(ExitPowerdown::EnableUtmiSuspend::SET);
    }

    /* // Which rails should we turn off?
        GR_PMU_LOW_POWER_DIS =
              GC_PMU_LOW_POWER_DIS_VDDIOF_MASK |
              GC_PMU_LOW_POWER_DIS_VDDXO_MASK |
              GC_PMU_LOW_POWER_DIS_JTR_RC_MASK;
     */
    registers.low_power_disable.write(LowPowerDisable::FlashOff::SET +
                                      LowPowerDisable::OscillatorOff::SET +
                                      LowPowerDisable::JitterRCOff::SET);

    //interrupt_disable();
    // Clear the RBOX wakeup signal and status bits
    // GREG32(RBOX, WAKEUP) = GC_RBOX_WAKEUP_CLEAR_MASK;
    // Wake on RBOX interrupts
    // GREG32(RBOX, WAKEUP) = GC_RBOX_WAKEUP_ENABLE_MASK;


    if deep {
        // __hw_clock_event_clear(); // really, no timers! the world of USB.

        unsafe {
            let pinmux = &mut *::pinmux::PINMUX;
            pinmux.dioa11.select.set(::pinmux::Function::Default);
            pinmux.dioa5.select.set(::pinmux::Function::Default);
            //pinmux.hold.set(1);

            //::gpio::PORT0.pins[0].clear();
            //::gpio::PORT0.pins[1].clear();
            //h1b::gpio::PORT0.pins[1].disable();


        /* Turn off all LEDS to save power during sleep */
        //SET_LED(GPIO_LED0, 0);
        //SET_LED(GPIO_LED1, 0);
        //SET_LED(GPIO_LED2, 0);
        //SET_LED(GPIO_LED3, 0);
        /* Ask RO to be silent (== faster) upon resume */
        //GWRITE_FIELD(PMU, LONG_LIFE_SCRATCH_WR_EN, REG1, 1);
        //GREG32(PMU, LONG_LIFE_SCRATCH1) |= BOARD_NO_RO_UART;
        //GWRITE_FIELD(PMU, LONG_LIFE_SCRATCH_WR_EN, REG1, 0);
        /* The USB PHY is our only wake-up source from deep-sleep */

        //GR_PMU_EXITPD_MASK = GC_PMU_EXITPD_MASK_UTMI_SUSPEND_N_MASK;

            registers.exitpd_mask.write(ExitPowerdown::EnableUtmiSuspend::SET);
            registers.peripheral_clocks1_enable.write(PeripheralClock1::Usb::SET);

    /*
            clock_enable_module(MODULE_USB, 1);

            if (!GREAD_FIELD(USB, PCGCCTL, RSTPDWNMODULE))
                usb_save_suspended_state();

            GREG32(PMU, PWRDN_SCRATCH17) =
                GREG32(PMU, PWRDN_SCRATCH17) + 1;

            GREG32(PINMUX, HOLD) = 1;

            GWRITE_FIELD(USB, PCGCCTL, PWRCLMP, 1);
            GWRITE_FIELD(USB, PCGCCTL, RSTPDWNMODULE, 1);
            GWRITE_FIELD(USB, PCGCCTL, STOPPCLK, 1);


            GR_PMU_LOW_POWER_DIS |=
                GC_PMU_LOW_POWER_DIS_VDDL_MASK;
         */

            //pinmux.hold.set(1);
            registers.peripheral_clocks1_enable.write(PeripheralClock1::Usb::SET);
            //::usb::USB0.power_down();
            registers.low_power_disable.modify(LowPowerDisable::VddlOff::SET);
        }
        unsafe {cortexm3::scb::set_sleepdeep();}
    } else {
        unsafe {cortexm3::scb::unset_sleepdeep();}
    }





}

pub fn resume_from_sleep() {
    let registers = &*PMU_REGISTERS;
    /* Prevent accidental reentry */
    /* Cr50 code does this, don't know why, but better safe than sorry. -pal */
    registers.low_power_disable.set(0);
    registers.exitpd_mask.set(0);

    unsafe {
        if SLEEP_DEEPLY {
            ::usb::USB0.init(&mut ::usb::EP0_OUT_DESCRIPTORS,
                             &mut ::usb::EP0_OUT_BUFFERS,
                             &mut ::usb::EP0_IN_DESCRIPTORS,
                             &mut ::usb::EP0_IN_BUFFER,
                             &mut ::usb::EP1_OUT_DESCRIPTOR,
                             &mut ::usb::EP1_OUT_BUFFER,
                             &mut ::usb::EP1_IN_DESCRIPTOR,
                             &mut ::usb::EP1_IN_BUFFER,
                             &mut ::usb::CONFIGURATION_BUFFER,
                             ::usb::PHY::A,
                             None,
                             Some(0x18d1),  // Google vendor ID
                             Some(0x5026),  // proto2
                             &mut ::usb::u2f::STRINGS);
            //print!("Leaving deep sleep.\n");
            ::usb::USB0.power_up();
            //::usb::USB0.resume(::usb::PHY::A);
            //print!("USB powered up.\n");
        }
    }
    disable_deep_sleep();
}
