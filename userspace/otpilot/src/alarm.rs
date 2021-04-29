// Copyright 2021 lowRISC contributors.
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
//
// SPDX-License-Identifier: Apache-2.0

use core::cell::Cell;

use libtock::result::TockResult;
use libtock::syscalls;

pub trait Alarm {
    // Get clock frequency in Hz.
    fn get_clock_frequency(&self) -> usize;

    // Set alarm to occur after `ticks`.
    fn set(&self, ticks: usize) -> TockResult<()>;

    // Check if the alarm is expired.
    fn is_expired(&self) -> bool;

    // Clear expired alarm or stop it if it's still running.
    fn clear(&self) -> TockResult<()>;
}

// Get the static Controller object.
pub fn get() -> &'static dyn Alarm {
    get_impl()
}

const DRIVER_NUMBER: usize = 0x00000;

mod command_nr {
    pub const CHECK_IF_PRESENT: usize = 0;
    pub const GET_CLOCK_FREQUENCY: usize = 1;
    pub const STOP_ALARM: usize = 3;
    pub const SET_RELATIVE_ALARM: usize = 5;
}

mod subscribe_nr {
    pub const ALARM_EXPIRED: usize = 0;
}

struct AlarmImpl {
    // Clock frequency for alarm
    clock_frequency: usize,

    // ID of running alarm
    alarm_id: Cell<Option<usize>>,

    // Whether the alarm is expired.
    alarm_expired: Cell<bool>,
}

static mut ALARM: AlarmImpl = AlarmImpl {
    clock_frequency: core::usize::MAX,
    alarm_id: Cell::new(None),
    alarm_expired: Cell::new(false),
};

static mut IS_INITIALIZED: bool = false;

fn get_impl() -> &'static AlarmImpl {
    unsafe {
        if !IS_INITIALIZED {
            if ALARM.initialize().is_err() {
                panic!("Could not initialize Alarm");
            }
            IS_INITIALIZED = true;
        }
        &ALARM
    }
}

impl AlarmImpl {
    fn initialize(&'static mut self) -> TockResult<()> {
        syscalls::command(DRIVER_NUMBER, command_nr::CHECK_IF_PRESENT, 0, 0)?;

        self.clock_frequency =
            syscalls::command(DRIVER_NUMBER, command_nr::GET_CLOCK_FREQUENCY, 0, 0)?;

        syscalls::subscribe_fn(
            DRIVER_NUMBER,
            subscribe_nr::ALARM_EXPIRED,
            AlarmImpl::alarm_expired_trampoline,
            0)?;

        Ok(())
    }

    extern "C"
    fn alarm_expired_trampoline(arg1: usize, arg2: usize, arg3: usize, _data: usize) {
        get_impl().alarm_expired(arg1, arg2, arg3);
    }

    fn alarm_expired(&self, _ticks: usize, id: usize, _: usize) {
        if let Some(alarm_id) = self.alarm_id.get() {
            if alarm_id == id {
                self.alarm_expired.set(true)
            }
        }
    }
}

impl Alarm for AlarmImpl {
    fn get_clock_frequency(&self) ->  usize {
        self.clock_frequency
    }

    fn set(&self, ticks: usize) -> TockResult<()> {
        self.alarm_expired.set(false);
        self.alarm_id.set(None);
        let alarm_id = syscalls::command(DRIVER_NUMBER, command_nr::SET_RELATIVE_ALARM, ticks, 0)?;
        self.alarm_id.set(Some(alarm_id));
        Ok(())
    }

    fn is_expired(&self) -> bool {
        self.alarm_id.get().is_some() && self.alarm_expired.get()
    }

    fn clear(&self) -> TockResult<()> {
        // Clear an expired alarm.
        if self.alarm_expired.get() {
            self.alarm_expired.set(false);
            self.alarm_id.set(None);

            // There's nothing else to do here.
            return Ok(());
        }

        // Stop a running alarm.
        if let Some(alarm_id) = self.alarm_id.get() {
            syscalls::command(DRIVER_NUMBER, command_nr::STOP_ALARM, alarm_id, 0)?;
            self.alarm_id.set(None);
        }


        Ok(())
    }
}
