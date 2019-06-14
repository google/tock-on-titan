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

/// Verifies its input is true, otherwise returns false. Similar to assert!(),
/// but returns false rather than panicking on failure.

#[macro_export]
macro_rules! require {
	($expr:expr) => (if !$expr {
		use core::fmt::Write;
		let _ = writeln!(libtock::console::Console::new(), "FAILED: {}", stringify!($expr));
		return false;
	});
	($expr:expr,) => (require!($expr));
}
