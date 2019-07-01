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

#[cfg(test)]
#[derive(Clone,Copy,PartialEq)]
enum MockClientState {
	EraseDone(kernel::ReturnCode),
	WriteDone(kernel::ReturnCode),
}

#[cfg(test)]
struct MockClient {
	state: core::cell::Cell<Option<MockClientState>>,
}

#[cfg(test)]
impl MockClient {
	pub fn new() -> Self {
		MockClient { state: core::cell::Cell::new(None) }
	}

	pub fn state(&self) -> Option<MockClientState> { self.state.get() }
}

#[cfg(test)]
impl h1b::hil::flash::Client for MockClient {
	fn erase_done(&self, code: kernel::ReturnCode) {
		self.state.set(Some(MockClientState::EraseDone(code)));
	}

	fn write_done(&self, code: kernel::ReturnCode) {
		self.state.set(Some(MockClientState::WriteDone(code)));
	}
}

#[test]
fn successful_program() -> bool {
	use { h1b::hil::flash::Hardware, kernel::hil::time::Alarm, test::require };
	let alarm = crate::hil::flash::mock_alarm::MockAlarm::new();
	let client = MockClient::new();
	let hw = h1b::hil::flash::fake::FakeHw::new();
	hw.set_transaction(1300, 1);
	hw.set_write_data(&[0xFFFF0FFF]);

	// First attempt.
	let driver = unsafe { h1b::hil::flash::Flash::new(&alarm, &hw) };
	hw.set_client(&driver);
	driver.set_client(&client);
	require!(driver.write(1300, &[0xFFFFABCD]) == kernel::ReturnCode::SUCCESS);
	require!(alarm.get_alarm() == alarm.now() + 150);
	require!(client.state() == None);
	require!(hw.is_programming() == true);

	// Inject a spurious interrupt.
	alarm.set_time(50);
	require!(alarm.get_alarm() == 150);
	require!(hw.is_programming() == true);
	require!(client.state() == None);

	// Indicate an error, let the driver retry.
	alarm.set_time(100);
	hw.inject_error(0b100);
	require!(alarm.get_alarm() == 250);
	require!(hw.is_programming() == true);
	require!(client.state() == None);

	// Let the operation finish successfully.
	alarm.set_time(200);
	hw.finish_operation();
	require!(alarm.get_alarm() == 0);
	require!(hw.is_programming() == false);
	require!(client.state() == Some(MockClientState::WriteDone(kernel::ReturnCode::SUCCESS)));

	true
}

#[test]
fn timeout() -> bool {
	use { h1b::hil::flash::Hardware, kernel::hil::time::{Alarm, Client}, test::require };
	let alarm = crate::hil::flash::mock_alarm::MockAlarm::new();
	let client = MockClient::new();
	let hw = h1b::hil::flash::fake::FakeHw::new();
	hw.set_transaction(1300, 1);
	hw.set_write_data(&[0xFFFF0FFF]);

	// First attempt.
	let driver = unsafe { h1b::hil::flash::Flash::new(&alarm, &hw) };
	driver.set_client(&client);
	require!(driver.write(1300, &[0xFFFFABCD]) == kernel::ReturnCode::SUCCESS);
	require!(alarm.get_alarm() == alarm.now() + 150);
	require!(client.state() == None);
	require!(hw.is_programming() == true);

	// Indicate a timeout.
	alarm.set_time(200);
	driver.fired();
	require!(alarm.get_alarm() == 0);
	require!(client.state() == Some(MockClientState::WriteDone(kernel::ReturnCode::FAIL)));

	true
}

#[test]
fn write_max_retries() -> bool {
	use { h1b::hil::flash::Hardware, kernel::hil::time::Alarm, test::require };
	let alarm = crate::hil::flash::mock_alarm::MockAlarm::new();
	let client = MockClient::new();
	let hw = h1b::hil::flash::fake::FakeHw::new();
	hw.set_transaction(1300, 1);
	hw.set_write_data(&[0xFFFF0FFF]);
	let driver = unsafe { h1b::hil::flash::Flash::new(&alarm, &hw) };
	hw.set_client(&driver);
	driver.set_client(&client);
	require!(driver.write(1300, &[0xFFFFABCD]) == kernel::ReturnCode::SUCCESS);
	require!(alarm.get_alarm() == alarm.now() + 150);
	require!(client.state() == None);
	require!(hw.is_programming() == true);

	for i in 1..9 {
		// Indicate an error, let the driver retry.
		alarm.set_time(100 * i);
		hw.inject_error(0b100);
		require!(alarm.get_alarm() == alarm.now() + 150);
		require!(hw.is_programming() == true);
		require!(client.state() == None);
	}

	// Last try.
	hw.inject_error(0b100);
	require!(alarm.get_alarm() == 0);
	require!(hw.is_programming() == false);
	require!(client.state() == Some(MockClientState::WriteDone(kernel::ReturnCode::FAIL)));

	true
}
