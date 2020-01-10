// Copyright 2018 Google LLC
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

use capsules::test::alarm::TestAlarm;
use kernel::hil::time::Alarm;
use h1b::timels;
#[allow(unused_imports)]

pub unsafe fn run_alarm() {
    let r = static_init_test_alarm();
    timels::TIMELS0.set_client(r);
    r.run();
}

unsafe fn static_init_test_alarm() -> &'static mut TestAlarm<'static, timels::Timels> {
    static_init!(
        TestAlarm<'static, timels::Timels>,
        TestAlarm::new(&timels::TIMELS0)
    )
}
