use kernel::common::take_cell::TakeCell;
use kernel::common::volatile_cell::VolatileCell;
use core::cell::Cell;
use kernel::hil::alarm::{Alarm, AlarmClient, Frequency};

const TIMELS0_BASE: *const Registers = 0x40540000 as *const Registers;
const TIMELS1_BASE: *const Registers = 0x40540040 as *const Registers;

pub static mut Timels0: Timels = Timels::new(TIMELS0_BASE);
pub static mut Timels1: Timels = Timels::new(TIMELS1_BASE);

struct Registers {
    pub control: VolatileCell<u32>,
    pub status: VolatileCell<u32>,
    pub load: VolatileCell<u32>,
    pub reload: VolatileCell<u32>,
    pub value: VolatileCell<u32>,
    pub step: VolatileCell<u32>,
    pub interrupt_enable: VolatileCell<u32>,
    pub interrupt_status: VolatileCell<u32>,
    pub interrupt_pending: VolatileCell<u32>,
    pub interrupt_ack: VolatileCell<u32>,
    pub interrupt_wakeup_ack: VolatileCell<u32>,
}

pub struct Timels {
    registers: *const Registers,
    client: TakeCell<&'static AlarmClient>,
    now: Cell<u32>,
}

impl Timels {
    const fn new(regs: *const Registers) -> Timels {
        Timels {
            registers: regs,
            client: TakeCell::empty(),
            now: Cell::new(0),
        }
    }

    pub fn set_client(&'static self, client: &'static AlarmClient) {
        self.client.put(Some(client));
    }

    pub fn handle_interrupt(&self) {
        let regs = unsafe { &*self.registers };
        regs.interrupt_ack.set(1);
        regs.interrupt_wakeup_ack.set(1);
        regs.control.set(0);
        self.now.set(self.now.get().wrapping_add(regs.reload.get()));
        regs.reload.set(0);
        self.client.map(|client| {
            client.fired();
        });
    }
}

pub struct Freq256Khz;

impl Frequency for Freq256Khz {
    fn frequency() -> u32 {
        256000
    }
}

impl Alarm for Timels {
    type Frequency = Freq256Khz;

    fn now(&self) -> u32 {
        let regs = unsafe { &*self.registers };
        let cur = regs.value.get();
        let reload = regs.reload.get();
        let elapsed = reload - cur;
        self.now.get().wrapping_add(elapsed)
    }

    fn set_alarm(&self, tics: u32) {
        let distance = tics.wrapping_sub(self.now.get());
        let regs = unsafe { &*self.registers };
        regs.load.set(distance);
        regs.reload.set(distance);
        regs.interrupt_enable.set(1);
        regs.control.set(1);
    }

    fn disable_alarm(&self) {
        let regs = unsafe { &*self.registers };
        regs.control.set(0);
    }

    fn is_armed(&self) -> bool {
        let regs = unsafe { &*self.registers };
        regs.control.get() & 1 == 1 && regs.value.get() != 0
    }

    fn get_alarm(&self) -> u32 {
        let regs = unsafe { &*self.registers };
        regs.reload.get()
    }
}
