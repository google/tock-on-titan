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

pub struct MockAlarm {
    current_time: core::cell::Cell<kernel::hil::time::Ticks32>,
    setpoint: core::cell::Cell<Option<kernel::hil::time::Ticks32>>,
}

impl MockAlarm {
    pub fn new() -> MockAlarm {
        MockAlarm {
            current_time: core::cell::Cell::new(0.into()),
            setpoint: core::cell::Cell::new(Some(0.into())),
        }
    }

    pub fn set_time(&self, new_time: kernel::hil::time::Ticks32) { self.current_time.set(new_time); }
}

impl kernel::hil::time::Time for MockAlarm {
    type Frequency = h1::timels::Freq256Khz;
    type Ticks = kernel::hil::time::Ticks32;

    fn now(&self) -> Self::Ticks { self.current_time.get() }
}

impl<'a> kernel::hil::time::Alarm<'a> for MockAlarm {
    fn set_alarm(&self, reference: Self::Ticks, dt: Self::Ticks) {
        use kernel::hil::time::Ticks;
        self.setpoint.set(Some(reference.wrapping_add(dt)));
    }

    fn get_alarm(&self) -> Self::Ticks { self.setpoint.get().unwrap_or(0.into()) }

    // Ignored -- the test should manually trigger the client.
    fn set_alarm_client(&'a self, _client: &'a dyn kernel::hil::time::AlarmClient) {}

    fn is_armed(&self) -> bool { self.setpoint.get().is_some() }

    fn disarm(&self) -> kernel::ReturnCode {
        self.setpoint.set(None);
        kernel::ReturnCode::SUCCESS
    }

    fn minimum_dt(&self) -> Self::Ticks { 1.into() }
}
