// Copyright 2019 Google LLC
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

use h1::hil::flash::driver::WRITE_OPCODE;
use h1::hil::flash::{Bank,Hardware,smart_program};
use kernel::hil::time::{Alarm,Frequency,Ticks,Time};
use super::mock_alarm::MockAlarm;
use test::require;

#[test]
fn decode_error() -> bool {
    require!(smart_program::decode_error(0b100000000101001) == kernel::ReturnCode::FAIL);
    require!(smart_program::decode_error(0b100000000000010) == kernel::ReturnCode::ESIZE);
    require!(smart_program::decode_error(0b000000000001000) == kernel::ReturnCode::FAIL);
    true
}

#[test]
fn div_round_up() -> bool {
    require!(smart_program::div_round_up(0, 1) == 0);
    require!(smart_program::div_round_up(1, 1) == 1);
    require!(smart_program::div_round_up(3, 2) == 2);
    require!(smart_program::div_round_up(4, 2) == 2);
    require!(smart_program::div_round_up(0, core::u64::MAX) == 0);
    require!(smart_program::div_round_up(core::u64::MAX, core::u64::MAX) == 1);
    require!(smart_program::div_round_up(core::u64::MAX - 5, core::u64::MAX) == 1);
    require!(smart_program::div_round_up(core::u64::MAX, core::u64::MAX) == 1);
    true
}

#[test]
fn successful_program() -> bool {
    let alarm = MockAlarm::new();
    let hw = h1::hil::flash::fake::FakeHw::new();
    hw.set_transaction(1300, 1);
    hw.set_write_data(&[0xFFFF0FFF]);

    // First attempt.
    let mut state = smart_program::SmartProgramState::init(8, true, 100_000_000);
    state = state.step(&alarm, &hw, WRITE_OPCODE, Bank::One);
    require!(alarm.get_alarm() == alarm.now().wrapping_add((<MockAlarm as Time>::Frequency::frequency()/10).into()));
    require!(hw.is_programming() == true);
    require!(state.return_code() == None);

    // Finish.
    alarm.set_time((<MockAlarm as Time>::Frequency::frequency()/10).into());
    hw.inject_result(0);
    state = state.step(&alarm, &hw, WRITE_OPCODE, Bank::One);
    require!(alarm.get_alarm() == (<MockAlarm as Time>::Frequency::frequency()/5).into());
    require!(hw.is_programming() == true);
    require!(state.return_code() == None);
    alarm.set_time((<MockAlarm as Time>::Frequency::frequency()/5).into());
    hw.finish_operation();
    state = state.step(&alarm, &hw, WRITE_OPCODE, Bank::One);
    require!(alarm.get_alarm() == 0.into());
    require!(hw.is_programming() == false);
    require!(state.return_code() == Some(kernel::ReturnCode::SUCCESS));
    true
}

#[test]
fn retries() -> bool {
    let alarm = MockAlarm::new();
    let hw = h1::hil::flash::fake::FakeHw::new();
    hw.set_transaction(1300, 1);
    hw.set_write_data(&[0xFFFF0FFF]);

    // First programming attempt.
    let mut state = smart_program::SmartProgramState::init(8, true, 100_000_000);
    state = state.step(&alarm, &hw, WRITE_OPCODE, Bank::One);
    require!(alarm.get_alarm() == alarm.now().wrapping_add((<MockAlarm as Time>::Frequency::frequency()/10).into()));
    require!(hw.is_programming() == true);
    require!(state.return_code() == None);
    alarm.set_time((<MockAlarm as Time>::Frequency::frequency()/10).into());
    hw.inject_result(0b100);

    // Second attempt.
    state = state.step(&alarm, &hw, WRITE_OPCODE, Bank::One);
    require!(alarm.get_alarm() == alarm.now().wrapping_add((<MockAlarm as Time>::Frequency::frequency()/10).into()));
    require!(hw.is_programming() == true);
    require!(state.return_code() == None);
    alarm.set_time((<MockAlarm as Time>::Frequency::frequency()/5).into());
    hw.inject_result(0);

    // Finish.
    state = state.step(&alarm, &hw, WRITE_OPCODE, Bank::One);
    require!(alarm.get_alarm() == (3*<MockAlarm as Time>::Frequency::frequency()/10).into());
    require!(hw.is_programming() == true);
    require!(state.return_code() == None);
    alarm.set_time((3*<MockAlarm as Time>::Frequency::frequency()/5).into());
    hw.finish_operation();
    state = state.step(&alarm, &hw, WRITE_OPCODE, Bank::One);
    require!(alarm.get_alarm() == 0.into());
    require!(hw.is_programming() == false);
    require!(state.return_code() == Some(kernel::ReturnCode::SUCCESS));
    true
}

/// Keep throwing errors until the max retry count.
#[test]
fn failed() -> bool {
    let alarm = MockAlarm::new();
    let hw = h1::hil::flash::fake::FakeHw::new();
    hw.set_transaction(1300, 1);
    hw.set_write_data(&[0xFFFF0FFF]);
    let mut state = smart_program::SmartProgramState::init(8, true, 100_000_000);

    for i in 0..8 {
        alarm.set_time((i*<MockAlarm as Time>::Frequency::frequency()/10).into());
        state = state.step(&alarm, &hw, WRITE_OPCODE, Bank::One);
        require!(alarm.get_alarm() ==
                 alarm.now().wrapping_add((<MockAlarm as Time>::Frequency::frequency()/10).into()));
        require!(hw.is_programming() == true);
        require!(state.return_code() == None);
        hw.inject_result(0b100);
    }

    // Finish.
    state = state.step(&alarm, &hw, WRITE_OPCODE, Bank::One);
    require!(alarm.get_alarm() == 0.into());
    require!(hw.is_programming() == false);
    require!(state.return_code() == Some(kernel::ReturnCode::FAIL));
    true
}

#[test]
fn timeout() -> bool {
    let alarm = MockAlarm::new();
    let hw = h1::hil::flash::fake::FakeHw::new();
    hw.set_transaction(1300, 1);
    hw.set_write_data(&[0xFFFF0FFF]);

    // First programming attempt.
    let mut state = smart_program::SmartProgramState::init(8, true, 100_000_000);
    state = state.step(&alarm, &hw, WRITE_OPCODE, Bank::One);
    require!(alarm.get_alarm() == alarm.now().wrapping_add((<MockAlarm as Time>::Frequency::frequency()/10).into()));
    require!(hw.is_programming() == true);
    require!(state.return_code() == None);

    // Alarm trigger.
    alarm.set_time((<MockAlarm as Time>::Frequency::frequency()/10).into());
    state = state.step(&alarm, &hw, WRITE_OPCODE, Bank::One);
    require!(alarm.get_alarm() == 0.into());
    require!(state.return_code() == Some(kernel::ReturnCode::FAIL));
    true
}
