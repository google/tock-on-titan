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

use kernel::ReturnCode;
use LastCallback::*;

#[derive(Debug,PartialEq)]
enum LastCallback {
    Uncalled,
    InitializeDone(ReturnCode),
    IncrementDone(ReturnCode),
}

impl core::default::Default for LastCallback {
    fn default() -> LastCallback {
        LastCallback::Uncalled
    }
}

struct MockClient {
    last_callback: core::cell::Cell<LastCallback>,
}

impl MockClient {
    pub fn new() -> MockClient {
        MockClient { last_callback: Default::default() }
    }

    pub fn take_last(&self) -> LastCallback {
        self.last_callback.take()
    }
}

impl h1::nvcounter::Client for MockClient {
    fn initialize_done(&self, status: ReturnCode) {
        self.last_callback.set(InitializeDone(status));
    }

    fn increment_done(&self, status: ReturnCode) {
        self.last_callback.set(IncrementDone(status));
    }
}


#[test]
fn test_capsule() -> bool {
    use crate::fake_flash::{ErrorTime,FakeFlash};
    use h1::hil::flash::flash::{Client,Flash};
    use h1::nvcounter::{FlashCounter,NvCounter};
    use h1::nvcounter::internal::{COUNTS_PER_PAGE,Page,WORDS_PER_PAGE};
    use ReturnCode::{EBUSY,FAIL,SUCCESS,SuccessWithValue};
    use test::{require,require_eq};

    // Setup
    let mut buffer = [0];
    let flash = FakeFlash::new();
    let nvcounter = FlashCounter::new(&mut buffer, &flash);
    let client = MockClient::new();
    nvcounter.set_client(&client);
    // Flip some bits so that initialization doesn't finish immediately after
    // step A1
    let mut buffer = [0];
    flash.write(Page::High as usize * WORDS_PER_PAGE + 100, &mut buffer);

    // Try to initialize the counter but fail the first erase call.
    flash.configure_error(Some(ErrorTime::Fast));
    require!(nvcounter.initialize() == FAIL);
    // Check to make sure it didn't mark the initialization as ongoing.
    require!(nvcounter.initialize() == FAIL);

    // Try to initialize again but make the first erase fail asynchronously.
    flash.configure_error(Some(ErrorTime::Callback));
    require!(nvcounter.initialize() == SUCCESS);
    // Confirm it will reject concurrent requests.
    require!(nvcounter.initialize() == EBUSY);
    require!(nvcounter.read_and_increment() == EBUSY);
    require!(client.take_last() == Uncalled);
    nvcounter.erase_done(FAIL);
    require!(client.take_last() == InitializeDone(FAIL));

    // Complete step A1; make the start of step A2 fail.
    flash.configure_error(None);
    require!(nvcounter.initialize() == SUCCESS);
    flash.configure_error(Some(ErrorTime::Fast));
    require!(client.take_last() == Uncalled);
    nvcounter.erase_done(SUCCESS);
    require!(client.take_last() == InitializeDone(FAIL));

    // Restart initialization, and make step A2 fail asynchronously.
    flash.configure_error(None);
    require!(nvcounter.initialize() == SUCCESS);
    flash.configure_error(Some(ErrorTime::Callback));
    nvcounter.erase_done(SUCCESS);
    require!(client.take_last() == Uncalled);
    nvcounter.erase_done(FAIL);
    require!(client.take_last() == InitializeDone(FAIL));

    // Successful initialization.
    flash.configure_error(None);
    require!(nvcounter.initialize() == SUCCESS);
    require!(client.take_last() == Uncalled);
    nvcounter.erase_done(SUCCESS);
    require!(client.take_last() == Uncalled);
    nvcounter.erase_done(SUCCESS);
    require!(client.take_last() == InitializeDone(SUCCESS));

    // Perform a successful read and increment.
    require!(nvcounter.read_and_increment() == SuccessWithValue { value: 0 });
    require!(client.take_last() == Uncalled);
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, SUCCESS);
    require!(client.take_last() == IncrementDone(SUCCESS));

    // Try to increment but make the initial write call fail.
    flash.configure_error(Some(ErrorTime::Fast));
    require!(nvcounter.read_and_increment() == FAIL);
    require!(client.take_last() == Uncalled);

    // Try to increment; fail the write call asynchronously.
    flash.configure_error(Some(ErrorTime::Callback));
    require!(nvcounter.read_and_increment() == SuccessWithValue { value: 1 });
    require!(client.take_last() == Uncalled);
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, FAIL);
    require!(client.take_last() == IncrementDone(FAIL));

    // Adjust the flash state to be two ticks before low page rollover.
    flash.configure_error(None);
    let mut buffer = [0x0000003C];
    flash.write(Page::Low as usize * WORDS_PER_PAGE + 511, &mut buffer);

    // Increment. This should leave the flash in the state immediately before
    // low page rollover.
    require!(nvcounter.read_and_increment() ==
             SuccessWithValue { value: COUNTS_PER_PAGE as usize - 1 });
    // Confirm it will reject concurrent requests.
    require!(nvcounter.initialize() == EBUSY);
    require!(nvcounter.read_and_increment() == EBUSY);
    require!(client.take_last() == Uncalled);
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, SUCCESS);
    require!(client.take_last() == IncrementDone(SUCCESS));

    // Start the rollover increment; fail the first high page increment (C1)
    // immediately.
    flash.configure_error(Some(ErrorTime::Fast));
    require!(nvcounter.read_and_increment() == FAIL);
    require!(client.take_last() == Uncalled);

    // Start the rollover increment; fail the first high page increment (C1)
    // asynchronously.
    flash.configure_error(Some(ErrorTime::Callback));
    require_eq!("C1 async FAIL", nvcounter.read_and_increment(),
                SuccessWithValue { value: COUNTS_PER_PAGE as usize });
    require!(client.take_last() == Uncalled);
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, FAIL);
    require!(client.take_last() == IncrementDone(FAIL));

    // Start the rollover increment; let the high page increment succeed but
    // fail the low page erase quickly. This will commit the increment but not
    // clean it up, so we should get a successful call.
    flash.configure_error(None);
    require_eq!("C2 async FAIL", nvcounter.read_and_increment(),
                SuccessWithValue { value: COUNTS_PER_PAGE as usize });
    require!(client.take_last() == Uncalled);
    flash.configure_error(Some(ErrorTime::Fast));
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, SUCCESS);
    require!(client.take_last() == IncrementDone(SUCCESS));
    // Now the high page is odd and the low page is maxed out.

    // Try another increment. Fail step C2 asynchronously.
    flash.configure_error(Some(ErrorTime::Callback));
    require!(nvcounter.read_and_increment() ==
             SuccessWithValue { value: COUNTS_PER_PAGE as usize + 1 });
    require!(client.take_last() == Uncalled);
    nvcounter.erase_done(FAIL);
    require!(client.take_last() == IncrementDone(FAIL));

    // Try another increment, fail step C3 immediately.
    flash.configure_error(None);
    require_eq!("C3 fast FAIL", nvcounter.read_and_increment(),
                SuccessWithValue { value: COUNTS_PER_PAGE as usize + 1 });
    require!(client.take_last() == Uncalled);
    flash.configure_error(Some(ErrorTime::Fast));
    nvcounter.erase_done(SUCCESS);
    require!(client.take_last() == IncrementDone(FAIL));

    // Try to increment, fail step C3 asynchronously.
    flash.configure_error(Some(ErrorTime::Callback));
    require_eq!("C3 async FAIL", nvcounter.read_and_increment(),
                SuccessWithValue { value: COUNTS_PER_PAGE as usize + 1 });
    require!(client.take_last() == Uncalled);
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, FAIL);
    require!(client.take_last() == IncrementDone(FAIL));

    // Finish the rollover increment, and fail the next increment immediately.
    flash.configure_error(None);
    require_eq!("rollover1", nvcounter.read_and_increment(),
                SuccessWithValue { value: COUNTS_PER_PAGE as usize + 1 });
    require!(client.take_last() == Uncalled);
    flash.configure_error(Some(ErrorTime::Fast));
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, SUCCESS);
    require!(client.take_last() == IncrementDone(FAIL));

    // Perform a successful increment.
    flash.configure_error(None);
    require_eq!("post-rollover", nvcounter.read_and_increment(),
                SuccessWithValue { value: COUNTS_PER_PAGE as usize + 1 });
    require!(client.take_last() == Uncalled);
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, SUCCESS);
    require!(client.take_last() == IncrementDone(SUCCESS));

    // Advance to the next low page rollover and perform an error-free rollover
    // increment and cleanup.
    let mut buffer = [0];
    flash.write(Page::Low as usize * WORDS_PER_PAGE + 511, &mut buffer);
    require_eq!("rollover2", nvcounter.read_and_increment(),
                SuccessWithValue { value: 2 * COUNTS_PER_PAGE as usize + 1 });
    require!(client.take_last() == Uncalled);
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, SUCCESS);
    require!(client.take_last() == IncrementDone(SUCCESS));
    nvcounter.erase_done(SUCCESS);
    require!(client.take_last() == Uncalled);
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, SUCCESS);
    require!(client.take_last() == Uncalled);
    // Verify the value with another increment.
    require_eq!("post-rollover2", nvcounter.read_and_increment(),
                SuccessWithValue { value: 2 * COUNTS_PER_PAGE as usize + 2 });
    require!(client.take_last() == Uncalled);
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, SUCCESS);
    require!(client.take_last() == IncrementDone(SUCCESS));

    // Advance to the next rollover again, and perform an error-free rollover
    // increment with no delay before the next increment.
    let mut buffer = [0];
    flash.write(Page::Low as usize * WORDS_PER_PAGE + 511, &mut buffer);
    require_eq!("rollover3", nvcounter.read_and_increment(),
                SuccessWithValue { value: 3 * COUNTS_PER_PAGE as usize + 2 });
    require!(client.take_last() == Uncalled);
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, SUCCESS);
    require!(client.take_last() == IncrementDone(SUCCESS));
    // Note: The erase should still be going on, so make FakeFlash return EBUSY.
    flash.set_busy(true);
    require_eq!("post-rollover3", nvcounter.read_and_increment(),
                SuccessWithValue { value: 3 * COUNTS_PER_PAGE as usize + 3 });
    flash.set_busy(false);
    // Finish C2
    nvcounter.erase_done(SUCCESS);
    require!(client.take_last() == Uncalled);
    // Finish C3
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, SUCCESS);
    require!(client.take_last() == Uncalled);
    // Finish B1
    let mut buffer = [0];
    nvcounter.write_done(&mut buffer, SUCCESS);
    require!(client.take_last() == IncrementDone(SUCCESS));

    true
}
