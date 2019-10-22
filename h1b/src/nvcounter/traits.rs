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

use ::kernel::ReturnCode;

/// NvCounter traits. Must be made the flash's client before using either
/// initialize() or read_and_increment().

pub trait NvCounter<'c> {
    /// Initialize the counter to zero. Must be done once before
    /// incrementing the counter (this is persistent, not per-boot). Not
    /// atomic.
    fn initialize(&self) -> ReturnCode;

    /// Automically reads the counter and begins an increment operation. If
    /// successful, returns the pre-increment value. Will return EBUSY if an
    /// initialization or increment is ongoing. Note that callers must wait for
    /// a Client::increment_done call to know whether the operation succeeded.
    fn read_and_increment(&self) -> ReturnCode;

    fn set_client(&self, client: &'c dyn Client);
}

/// Trait to be implemented by NvCounter clients.
pub trait Client {
    /// Called when a counter-initialization operation finishes. Possible
    /// ReturnCode values:
    ///   SUCCESS  The initialization succeeded and the counter value is now 0
    ///   FAIL     The initialization failed and the counter has an arbitrary
    ///            value.
    fn initialize_done(&self, status: ReturnCode);

    /// Called when an increment operation completes. Possible ReturnCode value:
    ///   SUCCESS  The increment succeeded and the counter value is now 1 larger
    ///            than before.
    ///   FAIL     Something failed in the increment; the counter value probably
    ///            remains the same (but may have incremented by 1).
    ///   ESIZE    The counter is at its maximum value and cannot be incremented
    ///            further.
    fn increment_done(&self, status: ReturnCode);
}
