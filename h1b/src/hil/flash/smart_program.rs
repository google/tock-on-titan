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

/// A state machine for driving execution of the flash module's smart
/// programming functionality.

use ::kernel::hil::time::{Alarm,Frequency};
use ::kernel::ReturnCode;

pub enum SmartProgramState {
	Init(/*attempts_remaining*/ u8),
	Running(/*attempts_remaining*/ u8),
	Finished(/*return_code*/ ReturnCode),
}

use self::SmartProgramState::{Init,Finished,Running};

impl SmartProgramState {
	/// Initialize the smart programming state machine. The state machine must
	/// be stepped before it will do anything.
	pub fn init(max_attempts: u8) -> Self {
		Init(max_attempts)
	}

	/// Returns the return code for the smart program execution, or None if it
	/// is still running.
	pub fn return_code(&self) -> Option<ReturnCode> {
		if let Finished(code) = *self { Some(code) } else { None }
	}

	/// Performs a state machine update during smart programming. This should be
	/// done during initialization, after an interrupt, and when a timeout
	/// expires.
	pub fn step<'h, A: Alarm, H: super::hardware::Hardware<'h>>(
		self, alarm: &A, hw: &H, opcode: u32, is_timeout: bool) -> Self
	{
		match self {
			Init(attempts_remaining) => {
				hw.trigger(opcode);
				set_program_timeout(alarm);
				Running(attempts_remaining - 1)
			},
			Running(attempts_remaining) => {
				// Copied from Cr50: a timeout causes an immediate failure with
				// no retry.
				if is_timeout {
					alarm.disable();
					return Finished(ReturnCode::FAIL);
				}

				// If this was a spurious interrupt, ignore it.
				if hw.is_programming() { return Running(attempts_remaining); }

				// Check for a successful operation.
				let error = hw.read_error();
				if error == 0 {
					alarm.disable();
					return Finished(ReturnCode::SUCCESS);
				}

				// This programming attempt failed; retry if we haven't hit the
				// limit.
				if attempts_remaining > 0 {
					// Operation failed; retry.
					hw.trigger(opcode);
					set_program_timeout(alarm);
					return SmartProgramState::Running(attempts_remaining - 1);
				}

				// The operation failed max_attempts times -- indicate an error.
				alarm.disable();
				return SmartProgramState::Finished(decode_error(error));
			},
			Finished(return_code) => Finished(return_code),
		}
	}
}

/// Converts the given flash error flag value into a Tock kernel return code.
/// Assumes that error_flags is nonzero (e.g. that it represents a valid error).
pub fn decode_error(error_flags: u16) -> ReturnCode {
	// If the "out of main range" bit (0b10) is set, then the target location
	// was probably out of bounds. Otherwise, emit a generic error message (none
	// of the other error messages indicate anything other than driver or
	// hardware errors).
	if error_flags & 0b10 != 0 { ReturnCode::ESIZE } else { ReturnCode::FAIL }
}

// Divide two u32's while rounding up (rather than the default round-down
// behavior).
pub fn div_round_up(numerator: u32, denominator: u32) -> u32 {
	numerator / denominator + if numerator % denominator == 0 { 0 } else { 1 }
}

// Sets an alarm for 150ms in the future. 150ms comes from the Cr50 source code
// ({1} at the bottom of this file).
fn set_program_timeout<A: Alarm>(alarm: &A) {
	// 150ms is 3/20th of a second.
	alarm.set_alarm(alarm.now() + div_round_up(A::Frequency::frequency() * 3, 20));
}

// Links that are too long to inline:
//
// {1} https://chromium.googlesource.com/chromiumos/platform/ec/+/8a411be5297f9886e6ee8bf1fdac7fe6b7e53667/chip/g/flash.c#242
