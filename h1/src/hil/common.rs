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

/// Possible errors returned by syscalls. In case of failure, the negative value of the error is
/// returned to the application.
#[derive(Copy, Clone)]
pub enum SyscallError {
    /// Generic errors that haven't been updated to use a more specific code yet.
    Unknown = 1,
    /// An argument passed is not (and never is) valid for this particular call.
    InvalidArgument = 2,
    /// An argument passed or operation attempted is not valid for the current state of the object.
    InvalidState = 3,
    /// A numeric argument is out-of-range, or a passed buffer is too small.
    OutOfRange = 4,
    /// The requested operation is unknown or unsupported.
    NotImplemented = 5,
    /// The resource is currently busy.
    ResourceBusy = 6,
    /// Internal error in the kernel. This indicates a bug and that the kernel might be unstable.
    InternalError = 7,
}

impl From<SyscallError> for isize {
    fn from(e: SyscallError) -> Self {
        -(e as isize)
    }
}
