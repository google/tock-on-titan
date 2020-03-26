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

// Flash driver for H1. Implements the Tock flash HIL trait as well as an API
// more representative of the H1 flash hardware's capabilities (e.g. sub-page
// writes and counters).

pub mod driver;
#[cfg(feature = "test")]
pub mod fake;
pub mod flash;
pub mod h1_hw;
mod hardware;
pub mod smart_program;

#[cfg(feature = "test")]
pub type FlashImpl<'h, A> = self::driver::FlashImpl<'h, A, self::fake::FakeHw>;

 #[cfg(not(feature = "test"))]
pub type FlashImpl<'h, A> = self::driver::FlashImpl<'static, A, self::h1_hw::H1bHw>;

pub use self::flash::{Client,Flash};
pub use self::hardware::Hardware;

// Constants used by multiple submodules.
const WORDS_PER_PAGE: usize = 512;
