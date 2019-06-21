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

#[test]
fn decode_error() -> bool {
	use { h1b::hil::flash::smart_program, test::require };
	require!(smart_program::decode_error(0b100000000101001) == kernel::ReturnCode::FAIL);
	require!(smart_program::decode_error(0b100000000000010) == kernel::ReturnCode::ESIZE);
	require!(smart_program::decode_error(0b000000000001000) == kernel::ReturnCode::FAIL);
	true
}

#[test]
fn div_round_up() -> bool {
	use { h1b::hil::flash::smart_program, test::require };
	require!(smart_program::div_round_up(0, 1) == 0);
	require!(smart_program::div_round_up(1, 1) == 1);
	require!(smart_program::div_round_up(3, 2) == 2);
	require!(smart_program::div_round_up(4, 2) == 2);
	require!(smart_program::div_round_up(0, core::u32::MAX) == 0);
	require!(smart_program::div_round_up(core::u32::MAX, core::u32::MAX) == 1);
	require!(smart_program::div_round_up(core::u32::MAX - 5, core::u32::MAX) == 1);
	require!(smart_program::div_round_up(core::u32::MAX, core::u32::MAX) == 1);
	true
}

#[test]
fn successful_program() -> bool {
	use { h1b::hil::flash::Hardware, kernel::hil::time::Alarm, test::require };
	let alarm = crate::hil::flash::mock_alarm::MockAlarm::new();
	let hw = h1b::hil::flash::fake::FakeHw::new();
	hw.set_transaction(1300, 1);
	hw.set_write_data(&[0xFFFF0FFF]);

	// First attempt.
	let mut state = h1b::hil::flash::smart_program::SmartProgramState::init(9);
	state = state.step(&alarm, &hw, 0x27182818, /*is_timeout:*/ false);
	require!(alarm.get_alarm() == alarm.now() + 150);
	require!(hw.is_programming() == true);
	require!(state.return_code() == None);
	alarm.set_time(50);

	// Inject a spurious interrupt.
	state = state.step(&alarm, &hw, 0x27182818, /*is_timeout:*/ false);
	require!(alarm.get_alarm() == 150);
	require!(hw.is_programming() == true);
	require!(state.return_code() == None);
	alarm.set_time(100);
	hw.finish_operation();

	// Finish.
	state = state.step(&alarm, &hw, 0x27182818, /*is_timeout:*/ false);
	require!(alarm.get_alarm() == 0);
	require!(hw.is_programming() == false);
	require!(state.return_code() == Some(kernel::ReturnCode::SUCCESS));
	true
}

#[test]
fn retries() -> bool {
	use { h1b::hil::flash::Hardware, kernel::hil::time::Alarm, test::require };
	let alarm = crate::hil::flash::mock_alarm::MockAlarm::new();
	let hw = h1b::hil::flash::fake::FakeHw::new();
	hw.set_transaction(1300, 1);
	hw.set_write_data(&[0xFFFF0FFF]);

	// First programming attempt.
	let mut state = h1b::hil::flash::smart_program::SmartProgramState::init(9);
	state = state.step(&alarm, &hw, 0x27182818, /*is_timeout:*/ false);
	require!(alarm.get_alarm() == alarm.now() + 150);
	require!(hw.is_programming() == true);
	require!(state.return_code() == None);
	alarm.set_time(100);
	hw.inject_error(0b100);

	// Second attempt.
	state = state.step(&alarm, &hw, 0x27182818, /*is_timeout:*/ false);
	require!(alarm.get_alarm() == alarm.now() + 150);
	require!(hw.is_programming() == true);
	require!(state.return_code() == None);
	alarm.set_time(200);
	hw.finish_operation();

	// Finish.
	state = state.step(&alarm, &hw, 0x27182818, /*is_timeout:*/ false);
	require!(alarm.get_alarm() == 0);
	require!(hw.is_programming() == false);
	require!(state.return_code() == Some(kernel::ReturnCode::SUCCESS));
	true
}

/// Keep throwing errors until the max retry count.
#[test]
fn failed() -> bool {
	use { h1b::hil::flash::Hardware, kernel::hil::time::Alarm, test::require };
	let alarm = crate::hil::flash::mock_alarm::MockAlarm::new();
	let hw = h1b::hil::flash::fake::FakeHw::new();
	hw.set_transaction(1300, 1);
	hw.set_write_data(&[0xFFFF0FFF]);
	let mut state = h1b::hil::flash::smart_program::SmartProgramState::init(9);

	for i in 0..9 {
		alarm.set_time(100 * i);
		state = state.step(&alarm, &hw, 0x27182818, /*is_timeout:*/ false);
		require!(alarm.get_alarm() == alarm.now() + 150);
		require!(hw.is_programming() == true);
		require!(state.return_code() == None);
		hw.inject_error(0b100);
	}

	// Finish.
	state = state.step(&alarm, &hw, 0x27182818, /*is_timeout:*/ false);
	require!(alarm.get_alarm() == 0);
	require!(hw.is_programming() == false);
	require!(state.return_code() == Some(kernel::ReturnCode::FAIL));
	true
}

#[test]
fn timeout() -> bool {
	use { h1b::hil::flash::Hardware, kernel::hil::time::Alarm, test::require };
	let alarm = crate::hil::flash::mock_alarm::MockAlarm::new();
	let hw = h1b::hil::flash::fake::FakeHw::new();
	hw.set_transaction(1300, 1);
	hw.set_write_data(&[0xFFFF0FFF]);

	// First programming attempt.
	let mut state = h1b::hil::flash::smart_program::SmartProgramState::init(9);
	state = state.step(&alarm, &hw, 0x27182818, /*is_timeout:*/ false);
	require!(alarm.get_alarm() == alarm.now() + 150);
	require!(hw.is_programming() == true);
	require!(state.return_code() == None);

	// Timeout: reset hardware and advance the time.
	hw.inject_error(0);
	alarm.set_time(200);

	// Alarm trigger.
	state = state.step(&alarm, &hw, 0x27182818, /*is_timeout:*/ true);
	require!(alarm.get_alarm() == 0);
	require!(hw.is_programming() == false);
	require!(state.return_code() == Some(kernel::ReturnCode::FAIL));
	true
}
