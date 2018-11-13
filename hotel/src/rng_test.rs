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

use test_rng::TestRng;
use kernel::hil::rng::RNG;
use hotel::trng;
use hotel::test_rng;

pub unsafe fn run_rng() {
    let r = static_init_test_rng();
    trng::TRNG0.set_client(r);
    r.run();
}

unsafe fn static_init_test_rng() -> &'static mut TestRng<'static> {
    static_init!(
        TestRng<'static>,
        TestRng::new(&trng::TRNG0)
    )
}
