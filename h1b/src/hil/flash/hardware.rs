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

/// The interface between the flash driver and the (real or fake) flash module.

pub trait Hardware<'a> {
	/// Returns true if an operation is running, false otherwise.
	fn is_programming(&self) -> bool;

	/// Read a single word from the flash (non-blocking). offset is in units of
	/// words and is relative to the start of flash.
	fn read(&self, offset: usize) -> u32;

	/// Reads the flash error code.
	fn read_error(&self) -> u16;

	/// Sets the client (the job receiving interrupts from the underlying
	/// hardware).
	fn set_client(&self, client: &'a Client);

	/// Set flash transaction parameters (word offset and size). The word offset
	/// is relative to the start of flash and the size is one less than the
	/// number of words to copy.
	fn set_transaction(&self, offset: usize, size: usize);

	/// Fill the flash controller's write buffer. data must have a length no
	/// larger than 32.
	fn set_write_data(&self, data: &[u32]);

	/// Kick off a smart program execution.
	fn trigger(&self, opcode: u32);
}

trait Client {
	/// Called when a flash programming operation completes.
	fn interrupt(&self);
}
