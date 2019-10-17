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

/// Non-volatile counter driver. Implements the syscall API documented in
/// doc/nvcounter_syscalls.md. Must be made the client of the NvCounter capsule.

use h1b::nvcounter::NvCounter;
use kernel::{AppId,Callback,ReturnCode};

pub const DRIVER_NUM: usize = 0x80040000;

#[derive(Default)]
pub struct AppData {
    wants_increment: bool,
    callback: Option<kernel::Callback>,
}

pub struct NvCounterSyscall<'c, C: NvCounter<'c>> {
    current_app: core::cell::Cell<usize>,  // max_value() if no op ongoing.
    grant: kernel::Grant<AppData>,
    init_failed: core::cell::Cell<bool>,
    nvcounter: &'c C,
    value: core::cell::Cell<usize>,
}

impl<'c, C: NvCounter<'c>> NvCounterSyscall<'c, C> {
    pub fn new(nvcounter: &'c C, grant: kernel::Grant<AppData>) -> Self {
        NvCounterSyscall {
            current_app: core::cell::Cell::new(usize::max_value()),
            grant,
            init_failed: Default::default(),
            nvcounter,
            // value will be corrected when the first operation completes, and
            // is not used until afterwards.
            value: Default::default()
        }
    }

    /// Try to initialize the counter. This should be called before process
    /// startup. If the initialization is successful, then normal operations
    /// will commence when it completes. If the initialization fails, the
    /// counter will be poisoned and will become unable to operate. Worse, the
    /// value stored in flash becomes undefined, although it will likely be a
    /// value between 0 and the previous value.
    #[allow(unused)]
    pub fn initialize(&self) {
        if self.nvcounter.initialize() != ReturnCode::SUCCESS {
            self.handle_failed_init();
        }
    }

    /// Sends failures to all apps with outstanding increment requests and marks
    /// init_failed as true.
    fn handle_failed_init(&self) {
        self.init_failed.set(true);
        self.grant.each(|app_data| {
            if !app_data.wants_increment { return; }
            app_data.wants_increment = false;
            if let Some(mut callback) = app_data.callback {
                callback.schedule(0, 0, 0);
            }
        });
    }

    // Scans through the apps and starts the next increment, if any app wants an
    // increment. This will also call the callback for app callback_id with the
    // given callback code -- specify an id if usize::max_value() if no callback
    // is necessary.
    fn do_next_op(&self, callback_id: usize, callback_code: usize) {
        use ReturnCode::SuccessWithValue;
        // TODO: Fairness? This seems to be the common approach but it gives
        // priority to lower-numbered apps. Probably not an issue for this
        // particular driver because read_and_increment() shouldn't see much
        // contention.
        self.grant.each(|app_data| {
            if self.current_app.get() == usize::max_value() &&
               app_data.wants_increment
            {
                app_data.wants_increment = false;
                if let SuccessWithValue { value } =
                    self.nvcounter.read_and_increment()
                {
                    self.value.set(value);
                    self.current_app.set(app_data.appid().idx());
                } else if let Some(mut callback) = app_data.callback {
                    callback.schedule(0, 0, 0);
                }
            }

            if app_data.appid().idx() == callback_id {
                if let Some(mut callback) = app_data.callback {
                    callback.schedule(callback_code, self.value.get(), 0);
                }
            }
        });
    }

    fn read_and_increment(&self, app: AppId) -> ReturnCode {
        if self.init_failed.get() { return ReturnCode::FAIL; }
        let result = self.grant.enter(app, |app_data, _| {
            if app_data.wants_increment { return ReturnCode::EBUSY; }
            app_data.wants_increment = true;
            ReturnCode::SUCCESS
        }).unwrap_or(ReturnCode::ENOMEM);
        if result != ReturnCode::SUCCESS { return result; }
        if self.current_app.get() == usize::max_value() {
            if self.nvcounter.read_and_increment() != ReturnCode::SUCCESS {
                return ReturnCode::FAIL;
            }
            self.current_app.set(app.idx());
        }
        ReturnCode::SUCCESS
    }

    fn set_increment_callback(&self, callback: Option<Callback>, app: AppId) -> ReturnCode {
        self.grant.enter(app, |app_data, _| {
            app_data.callback = callback;
            ReturnCode::SUCCESS
        }).unwrap_or(ReturnCode::ENOMEM)
    }
}

impl<'c, C: NvCounter<'c>> kernel::Driver for NvCounterSyscall<'c, C> {
    fn command(&self, minor_num: usize, _: usize, _: usize, app: AppId) -> ReturnCode {
        match minor_num {
            0 => ReturnCode::SUCCESS,
            1 => self.read_and_increment(app),
            _ => ReturnCode::ENOSUPPORT,
        }
    }

    fn subscribe(&self, minor_num: usize, callback: Option<Callback>, app_id: AppId) -> ReturnCode {
        match minor_num {
            0 => self.set_increment_callback(callback, app_id),
            _ => ReturnCode::ENOSUPPORT,
        }
    }
}

impl<'c, C: NvCounter<'c>> h1b::nvcounter::Client for NvCounterSyscall<'c, C> {
    fn initialize_done(&self, status: ReturnCode) {
        if status == ReturnCode::SUCCESS {
            self.init_failed.set(false);
            self.value.set(0);
            self.do_next_op(usize::max_value(), 0);
        } else {
            self.handle_failed_init();
        }
    }

    fn increment_done(&self, status: ReturnCode) {
        let callback_app = self.current_app.get();
        self.current_app.set(usize::max_value());
        let mut callback_code = 1;
        if status == ReturnCode::SUCCESS {
            self.value.set(self.value.get() + 1);
            callback_code = 2;
        }
        self.do_next_op(callback_app, callback_code);
    }
}
