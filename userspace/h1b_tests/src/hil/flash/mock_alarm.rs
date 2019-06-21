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
pub struct MockAlarm {
	now: core::cell::Cell<u32>,
	setpoint: core::cell::Cell<Option<u32>>,
}

#[cfg(test)]
impl MockAlarm {
	pub fn new() -> MockAlarm {
		MockAlarm { now: Default::default(), setpoint: Default::default() }
	}

	pub fn set_time(&self, new_time: u32) { self.now.set(new_time); }
}

#[cfg(test)]
impl kernel::hil::time::Time for MockAlarm {
	type Frequency = kernel::hil::time::Freq1KHz;
	fn disable(&self) { self.setpoint.set(None); }
	fn is_armed(&self) -> bool { self.setpoint.get().is_some() }
}

#[cfg(test)]
impl kernel::hil::time::Alarm for MockAlarm {
	fn now(&self) -> u32 { self.now.get() }
	fn set_alarm(&self, tics: u32) { self.setpoint.set(Some(tics)); }
	fn get_alarm(&self) -> u32 { self.setpoint.get().unwrap_or(0) }
}
