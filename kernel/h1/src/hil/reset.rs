// Copyright 2021 lowRISC contributors.
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
//
// SPDX-License-Identifier: Apache-2.0

//! Interfaces for reset monitor and execution on H1

use spiutils::driver::reset::ResetSource;

pub trait Reset {
    /// Immediately reset chip.
    fn reset_chip(&self) -> !;

    /// Get source of the last reset.
    fn get_reset_source(&self) -> ResetSource;
}
