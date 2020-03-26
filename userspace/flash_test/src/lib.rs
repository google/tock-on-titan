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

#![no_std]

// Rust complains that things are unused if they are only used when cfg(test) is
// true. If we include modules when cfg(test) is false, then declarations in the
// modules need to be marked #[cfg(test)]. Instead, we simply do not include the
// code in other configs.

#[cfg(test)]
mod driver;
#[cfg(test)]
mod fake;
#[cfg(test)]
mod h1_hw;
#[cfg(test)]
mod mock_alarm;
#[cfg(test)]
mod smart_program;
