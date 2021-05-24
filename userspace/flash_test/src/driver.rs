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

use h1::hil::flash::{Flash,Hardware};
use kernel::hil::time::{Alarm,Ticks};
use kernel::ReturnCode;
use test::require;

// These are in counts of a 256 kHz clock.
#[cfg(test)]
const ERASE_TIME: u32 = 859;
#[cfg(test)]
const WRITE_WORD_TIME: u32 = 14;

static mut WRITE_BUF: [u32; 1] = [0; 1];

const WORDS_PER_BANK: usize = 0x10000;
const PAGES_PER_BANK: usize = 128;

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
impl<'a> MockClient {
    pub fn new() -> Self {
        MockClient { state: core::cell::Cell::new(None) }
    }

    pub fn state(&self) -> Option<MockClientState> {
        let state = self.state.get();
        self.state.set(None);
        state
    }
}

#[cfg(test)]
impl<'a> h1::hil::flash::Client<'a> for MockClient {
    fn erase_done(&self, code: kernel::ReturnCode) {
        self.state.set(Some(MockClientState::EraseDone(code)));
    }

    fn write_done(&self, _data: &'a mut [u32], code: kernel::ReturnCode) {
        self.state.set(Some(MockClientState::WriteDone(code)));
    }
}

#[test]
fn erase() -> bool {
    use kernel::hil::time::{AlarmClient,Time};
    let alarm = crate::mock_alarm::MockAlarm::new();
    let client = MockClient::new();
    let hw = h1::hil::flash::fake::FakeHw::new();

    // First attempt.
    let driver = unsafe { h1::hil::flash::FlashImpl::new(&alarm, &hw) };
    driver.set_client(&client);
    require!(driver.erase(2) == kernel::ReturnCode::SUCCESS);
    require!(alarm.get_alarm() == alarm.now().wrapping_add(ERASE_TIME.into()));
    require!(client.state() == None);
    require!(hw.is_programming() == true);

    // Indicate an error, let the driver retry.
    alarm.set_time(ERASE_TIME.into());
    hw.inject_result(0b100);
    driver.alarm();
    require!(alarm.get_alarm() == (2 * ERASE_TIME).into());
    require!(hw.is_programming() == true);
    require!(client.state() == None);

    // Let the operation finish successfully.
    alarm.set_time((2 * ERASE_TIME).into());
    hw.finish_operation();
    driver.alarm();
    require!(alarm.get_alarm() == 0.into());
    require!(hw.is_programming() == false);
    require!(hw.read(1300) == ReturnCode::SuccessWithValue { value: 0xFFFFFFFF });
    require!(client.state() == Some(MockClientState::EraseDone(kernel::ReturnCode::SUCCESS)));
    require!(driver.read(1300) == ReturnCode::SuccessWithValue { value: 0xFFFFFFFF });

    true
}

#[test]
fn erase_max_retries() -> bool {
    use kernel::hil::time::{AlarmClient,Time};
    let alarm = crate::mock_alarm::MockAlarm::new();
    let client = MockClient::new();
    let hw = h1::hil::flash::fake::FakeHw::new();
    let driver = unsafe { h1::hil::flash::FlashImpl::new(&alarm, &hw) };
    driver.set_client(&client);
    require!(driver.erase(2) == kernel::ReturnCode::SUCCESS);
    require!(alarm.get_alarm() == alarm.now().wrapping_add(ERASE_TIME.into()));
    require!(client.state() == None);
    require!(hw.is_programming() == true);

    for i in 1..45 {
        // Indicate an error, let the driver retry.
        alarm.set_time((ERASE_TIME * i).into());
        hw.inject_result(0b100);
        driver.alarm();
        require!(alarm.get_alarm() == alarm.now().wrapping_add(ERASE_TIME.into()));
        require!(hw.is_programming() == true);
        require!(client.state() == None);
    }

    // Last try.
    hw.inject_result(0b100);
    driver.alarm();
    require!(alarm.get_alarm() == 0.into());
    require!(hw.is_programming() == false);
    require!(client.state() == Some(MockClientState::EraseDone(kernel::ReturnCode::FAIL)));
    require!(hw.read(1300) == ReturnCode::SuccessWithValue { value: 0xFFFFFFFF });
    require!(driver.read(1300) == ReturnCode::SuccessWithValue { value: 0xFFFFFFFF });

    true
}

struct OperationsTest<'a> {
    alarm: &'a crate::mock_alarm::MockAlarm,
    client: &'a MockClient,
    hw: &'a h1::hil::flash::fake::FakeHw,
    driver: &'a h1::hil::flash::FlashImpl<'a, crate::mock_alarm::MockAlarm>,
}

impl<'a> OperationsTest<'a> {
    fn read(&self, address: usize, expected_val: usize) -> bool {
        require!(self.hw.read(address) == ReturnCode::SuccessWithValue { value: expected_val });
        require!(self.driver.read(address) == ReturnCode::SuccessWithValue { value: expected_val });

        true
    }

    fn write(&self, address: usize, val: usize) -> bool {
        use kernel::hil::time::{AlarmClient,Time};

        unsafe {
            WRITE_BUF[0] = val as u32;
            require!(self.driver.write(address, &mut WRITE_BUF) == (kernel::ReturnCode::SUCCESS, None));
        }
        require!(self.alarm.get_alarm() == self.alarm.now().wrapping_add(WRITE_WORD_TIME.into()));
        require!(self.client.state() == None);
        require!(self.hw.is_programming() == true);
        self.alarm.set_time(WRITE_WORD_TIME.into());
        self.hw.inject_result(0);
        self.driver.alarm();
        require!(self.hw.is_programming() == true);
        require!(self.client.state() == None);
        self.hw.finish_operation();
        self.driver.alarm();
        require!(self.alarm.get_alarm() == 0.into());
        require!(self.hw.is_programming() == false);
        require!(self.client.state() == Some(MockClientState::WriteDone(kernel::ReturnCode::SUCCESS)));

        true
    }

    fn erase(&self, page: usize) -> bool {
        use kernel::hil::time::{AlarmClient,Time};

        require!(self.driver.erase(page) == kernel::ReturnCode::SUCCESS);
        require!(self.alarm.get_alarm() == self.alarm.now().wrapping_add(ERASE_TIME.into()));
        require!(self.hw.is_programming() == true);
        self.alarm.set_time((WRITE_WORD_TIME + ERASE_TIME).into());
        self.hw.finish_operation();
        self.driver.alarm();
        require!(self.alarm.get_alarm() == 0.into());
        require!(self.hw.is_programming() == false);
        require!(self.client.state() == Some(MockClientState::EraseDone(kernel::ReturnCode::SUCCESS)));

        true
    }
}

#[test]
fn write_then_erase() -> bool {
    let alarm = crate::mock_alarm::MockAlarm::new();
    let client = MockClient::new();
    let hw = h1::hil::flash::fake::FakeHw::new();
    let driver = unsafe { h1::hil::flash::FlashImpl::new(&alarm, &hw) };
    driver.set_client(&client);
    let ops_test = OperationsTest {
        alarm: &alarm,
        client: &client,
        hw: &hw,
        driver: &driver,
    };

    // Write bank 0
    require!(ops_test.write(1300, 0xFFFFABCD));
    require!(ops_test.read(1300, 0xFFFFABCD));

    // Check that same address in bank 1 is untouched
    require!(ops_test.read(WORDS_PER_BANK + 1300, 0xFFFFFFFF));

    // Write bank 1
    require!(ops_test.write(WORDS_PER_BANK + 1300, 0x12349876));
    require!(ops_test.read(WORDS_PER_BANK + 1300, 0x12349876));

    // Check that same address in bank 0 is untouched
    require!(ops_test.read(1300, 0xFFFFABCD));

    // Erase bank 0
    require!(ops_test.erase(2));
    require!(ops_test.read(1300, 0xFFFFFFFF));

    // Check that same address in bank 1 is untouched
    require!(ops_test.read(WORDS_PER_BANK + 1300, 0x12349876));

    // Erase bank 1
    require!(ops_test.erase(PAGES_PER_BANK + 2));
    require!(ops_test.read(WORDS_PER_BANK + 1300, 0xFFFFFFFF));

    true
}

#[test]
fn write_to_bad_address() -> bool {
    let alarm = crate::mock_alarm::MockAlarm::new();
    let client = MockClient::new();
    let hw = h1::hil::flash::fake::FakeHw::new();

    // Write
    let driver = unsafe { h1::hil::flash::FlashImpl::new(&alarm, &hw) };
    driver.set_client(&client);

    unsafe {
        WRITE_BUF[0] = 0xFFFFABCD;
        require!(driver.write(0x100000, &mut WRITE_BUF) == (kernel::ReturnCode::EINVAL, Some(&mut WRITE_BUF)));
    }

    true
}

#[test]
fn successful_program() -> bool {
    use kernel::hil::time::{AlarmClient,Time};
    let alarm = crate::mock_alarm::MockAlarm::new();
    let client = MockClient::new();
    let hw = h1::hil::flash::fake::FakeHw::new();

    // First attempt.
    let driver = unsafe { h1::hil::flash::FlashImpl::new(&alarm, &hw) };
    driver.set_client(&client);

    unsafe {
        WRITE_BUF[0] = 0xFFFFABCD;
        require!(driver.write(1300, &mut WRITE_BUF) == (kernel::ReturnCode::SUCCESS, None));
    }
    require!(alarm.get_alarm() == alarm.now().wrapping_add(WRITE_WORD_TIME.into()));
    require!(client.state() == None);
    require!(hw.is_programming() == true);

    // Indicate an error, let the driver retry.
    alarm.set_time(WRITE_WORD_TIME.into());
    hw.inject_result(0b100);
    driver.alarm();
    require!(alarm.get_alarm() == (2 * WRITE_WORD_TIME).into());
    require!(hw.is_programming() == true);
    require!(client.state() == None);

    // Let the operation finish successfully (including the final pulse).
    alarm.set_time((2 * WRITE_WORD_TIME).into());
    hw.inject_result(0);
    driver.alarm();
    require!(alarm.get_alarm() == (3 * WRITE_WORD_TIME).into());
    require!(hw.is_programming() == true);
    require!(client.state() == None);
    hw.finish_operation();
    driver.alarm();
    require!(alarm.get_alarm() == 0.into());
    require!(hw.is_programming() == false);
    require!(client.state() == Some(MockClientState::WriteDone(kernel::ReturnCode::SUCCESS)));
    require!(driver.read(1300) == ReturnCode::SuccessWithValue { value: 0xFFFFABCD });

    true
}

#[test]
fn timeout() -> bool {
    use kernel::hil::time::{AlarmClient,Time};
    let alarm = crate::mock_alarm::MockAlarm::new();
    let client = MockClient::new();
    let hw = h1::hil::flash::fake::FakeHw::new();
    hw.set_transaction(1300, 1);
    hw.set_write_data(&[0xFFFF0FFF]);

    // First attempt.
    let driver = unsafe { h1::hil::flash::FlashImpl::new(&alarm, &hw) };
    driver.set_client(&client);

    unsafe {
        WRITE_BUF[0] = 0xFFFFABCD;
        require!(driver.write(1300, &mut WRITE_BUF) == (kernel::ReturnCode::SUCCESS, None));
    }
    require!(alarm.get_alarm() == alarm.now().wrapping_add(WRITE_WORD_TIME.into()));
    require!(client.state() == None);
    require!(hw.is_programming() == true);

    // Indicate a timeout.
    alarm.set_time(WRITE_WORD_TIME.into());
    driver.alarm();
    require!(alarm.get_alarm() == 0.into());
    require!(client.state() == Some(MockClientState::WriteDone(kernel::ReturnCode::FAIL)));

    true
}

#[test]
fn write_max_retries() -> bool {
    use kernel::hil::time::{AlarmClient,Time};
    let alarm = crate::mock_alarm::MockAlarm::new();
    let client = MockClient::new();
    let hw = h1::hil::flash::fake::FakeHw::new();
    let driver = unsafe { h1::hil::flash::FlashImpl::new(&alarm, &hw) };
    driver.set_client(&client);

    unsafe {
        WRITE_BUF[0] = 0xFFFFABCD;
        require!(driver.write(1300, &mut WRITE_BUF) == (kernel::ReturnCode::SUCCESS, None));
    }
    require!(alarm.get_alarm() == alarm.now().wrapping_add(WRITE_WORD_TIME.into()));
    require!(client.state() == None);
    require!(hw.is_programming() == true);

    for _ in 1..8 {
        // Indicate an error, let the driver retry.
        alarm.set_time((100 * WRITE_WORD_TIME).into());
        hw.inject_result(0b100);
        driver.alarm();
        require!(alarm.get_alarm() == alarm.now().wrapping_add(WRITE_WORD_TIME.into()));
        require!(hw.is_programming() == true);
        require!(client.state() == None);
    }

    // Last try.
    hw.inject_result(0b100);
    driver.alarm();
    require!(alarm.get_alarm() == 0.into());
    require!(hw.is_programming() == false);
    require!(client.state() == Some(MockClientState::WriteDone(kernel::ReturnCode::FAIL)));

    true
}
