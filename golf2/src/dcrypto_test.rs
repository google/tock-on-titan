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

use h1b::test_dcrypto::TestDcrypto;
use h1b::crypto::dcrypto;
#[allow(unused_imports)]
use h1b::crypto::dcrypto::{Dcrypto, DcryptoClient, DcryptoEngine};

pub unsafe fn run_dcrypto() {
    let r = static_init_test_dcrypto();
    dcrypto::DCRYPTO.set_client(r);
    r.run();
}

unsafe fn static_init_test_dcrypto() -> &'static mut TestDcrypto<'static> {
    static_init!(
        TestDcrypto<'static>,
        TestDcrypto::new(&dcrypto::DCRYPTO)
    )
}
