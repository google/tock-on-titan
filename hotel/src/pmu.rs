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

use kernel::common::volatile_cell::VolatileCell;
use core::mem::transmute;

/// Registers for the Power Management Unit (PMU)
// Non-public fields prefixed with "_" mark unused registers
#[repr(C, packed)]
pub struct PMURegisters {
    _reset: VolatileCell<u32>,
    _set_reset: VolatileCell<u32>,

    /// Clear register for the reset source
    pub clear_reset: VolatileCell<u32>,

    /// Status for source of last reset event
    ///
    /// Bits 0-7 correspond to:
    ///
    /// | bit | Description                                       |
    /// | --- | :------------------------------------------------ |
    /// | 0   | Power on reset                                    |
    /// | 1   | Low power exit                                    |
    /// | 2   | Watchdog reset                                    |
    /// | 3   | Lockup reset                                      |
    /// | 4   | SYSRESET                                          |
    /// | 5   | Software initiated reset through PMU_GLOBAL_RESET |
    /// | 6   | Fast burnout circuit                              |
    /// | 7   | Security breach reset                             |
    ///
    pub reset_source: VolatileCell<u32>,

    /// Global chip reset
    ///
    /// Initiates a reset of the system similar to toggling the external reset
    /// pin. To initiate a reset, write the key 0x7041776 to this register.
    pub global_reset: VolatileCell<u32>,

    pub low_power_disable: VolatileCell<u32>,

    pub low_power_bypass: VolatileCell<u32>,

    pub low_power_bypass_value: VolatileCell<u32>,

    pub set_wakeup_interrupt_controller: VolatileCell<u32>,

    pub clear_wakeup_interrupt_controller: VolatileCell<u32>,

    /// Value of the system vector table offset
    pub sysvtor: VolatileCell<u32>,

    /// Enable PMU to gate some clocks when processor is sleeping
    pub nap_enable: VolatileCell<u32>,

    _pmu_sw_pdb: VolatileCell<u32>,
    _pmu_sw_pdb_secure: VolatileCell<u32>,
    _pmu_vref: VolatileCell<u32>,
    _xtl_osc_bypass: VolatileCell<u32>,
    _flash_tm0_test_en_bypass: VolatileCell<u32>,

    /// Battery level indicator
    ///
    /// When non-zero, the voltage level is higher than specified in the vref
    /// register's BATMON field.
    pub battery_level_ok: VolatileCell<u32>,

    _b_reg_dig_ctrl: VolatileCell<u32>,
    _exitpd_mask: VolatileCell<u32>,
    _exitpd_src: VolatileCell<u32>,
    _exitpd_mon: VolatileCell<u32>,
    _osc_ctrl: VolatileCell<u32>,

    /// Turn on clocks for memory
    ///
    /// Bits 0-6 correspond to memory banks 0-6, respectively.
    pub memory_clk_set: VolatileCell<u32>,

    /// Turn off clocks for memory
    ///
    /// Bits 0-6 correspond to memory banks 0-6, respectively.
    pub memory_clk_clear: VolatileCell<u32>,

    /// Enable peripheral clocks (bank 0).
    ///
    /// Each bit corresponds to a different peripheral clock.
    pub peripheral_clocks0_enable: VolatileCell<u32>,

    /// Disable peripheral clocks (bank 0).
    ///
    /// Each bit corresponds to a different peripheral clock.
    pub peripheral_clocks0_disable: VolatileCell<u32>,

    /// Enable peripheral clocks (bank 1).
    ///
    /// Each bit corresponds to a different peripheral clock.
    pub peripheral_clocks1_enable: VolatileCell<u32>,

    /// Disable peripheral clocks (bank 1).
    ///
    /// Each bit corresponds to a different peripheral clock.
    pub peripheral_clocks1_disable: VolatileCell<u32>,
}

const PMU_BASE: isize = 0x40000000;

static mut PMU: *mut PMURegisters = PMU_BASE as *mut PMURegisters;

#[derive(Clone,Copy)]
pub enum PeripheralClock0 {
    Camo0,
    Crpyto0,
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
pub enum PeripheralClock1 {
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
    Bank0(PeripheralClock0),
    Bank1(PeripheralClock1),
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
        let pmu: &mut PMURegisters = unsafe { transmute(PMU) };
        match self.clock {
            PeripheralClock::Bank0(clock) => {
                pmu.peripheral_clocks0_enable.set(1 << (clock as u32));
            }
            PeripheralClock::Bank1(clock) => {
                pmu.peripheral_clocks1_enable.set(1 << (clock as u32));
            }
        }
    }

    pub fn disable(&self) {
        let pmu: &mut PMURegisters = unsafe { transmute(PMU) };
        match self.clock {
            PeripheralClock::Bank0(clock) => {
                pmu.peripheral_clocks0_disable.set(1 << (clock as u32));
            }
            PeripheralClock::Bank1(clock) => {
                pmu.peripheral_clocks1_disable.set(1 << (clock as u32));
            }
        }
    }
}
