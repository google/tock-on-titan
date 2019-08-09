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
	Init(/*attempts_remaining*/ u8, /*final_pulse_needed*/ bool, /*timeout_nanoseconds*/ u32),
	Running(/*attempts_remaining*/ u8, /*final_pulse_needed*/ bool, /*timeout_nanoseconds*/ u32),
	Finished(/*return_code*/ ReturnCode),
}

use self::SmartProgramState::{Init,Finished,Running};

impl SmartProgramState {
	/// Initialize the smart programming state machine. The state machine must
	/// be stepped before it will do anything.
	pub fn init(max_attempts: u8, final_pulse_needed: bool, timeout_nanoseconds: u32) -> Self {
		Init(max_attempts, final_pulse_needed, timeout_nanoseconds)
	}

	/// Returns the return code for the smart program execution, or None if it
	/// is still running.
	pub fn return_code(&self) -> Option<ReturnCode> {
		if let Finished(code) = *self { Some(code) } else { None }
	}

	/// Performs a state machine update during smart programming. This should be
	/// done during initialization and when a wait finishes.
	pub fn step<A: Alarm, H: super::hardware::Hardware>(
		self, alarm: &A, hw: &H, opcode: u32) -> Self
	{
		match self {
			Init(attempts_remaining, final_pulse_needed, timeout_nanoseconds) => {
				hw.trigger(opcode);
				set_program_timeout(alarm, timeout_nanoseconds);
				Running(attempts_remaining - 1, final_pulse_needed, timeout_nanoseconds)
			},
			Running(attempts_remaining, final_pulse_needed, timeout_nanoseconds) => {
				// Copied from Cr50: a timeout causes an immediate failure with
				// no retry.
				if hw.is_programming() {
					alarm.disable();
					return Finished(ReturnCode::FAIL);
				}

				// Check for a successful operation.
				let error = hw.read_error();
				if error == 0 {
					// If final_pulse_needed, trigger one last smart programming
					// cycle. Otherwise indicate success.
					if final_pulse_needed {
						hw.trigger(opcode);
						set_program_timeout(alarm, timeout_nanoseconds);
						return Running(0, false, timeout_nanoseconds);
					}
					alarm.disable();
					return Finished(ReturnCode::SUCCESS);
				}

				// This programming attempt failed; retry if we haven't hit the
				// limit.
				if attempts_remaining > 0 {
					// Operation failed; retry.
					hw.trigger(opcode);
					set_program_timeout(alarm, timeout_nanoseconds);
					return SmartProgramState::Running(attempts_remaining - 1,
						final_pulse_needed, timeout_nanoseconds);
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
pub fn div_round_up(numerator: u64, denominator: u64) -> u64 {
	numerator / denominator + if numerator % denominator == 0 { 0 } else { 1 }
}

fn set_program_timeout<A: Alarm>(alarm: &A, timeout_nanoseconds: u32) {
	alarm.set_alarm(alarm.now().wrapping_add(
		div_round_up(A::Frequency::frequency() as u64 * timeout_nanoseconds as u64,
		             1_000_000_000) as u32));
}
