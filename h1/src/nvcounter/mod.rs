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

/// Non-volatile counter capsule. Implements a global no-rollback counter using
/// the last two pages of flash.

mod capsule;
mod traits;

// Export the ::internal module if we're being tested to make the internal
// methods unit-testable.
#[cfg(feature = "test")]
pub mod internal;
#[cfg(not(feature = "test"))]
mod internal;

pub use self::capsule::FlashCounter;
pub use self::traits::{Client,NvCounter};
