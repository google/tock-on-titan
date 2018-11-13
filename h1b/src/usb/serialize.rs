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

use core::intrinsics::copy_nonoverlapping;
use core::mem::{transmute, size_of};

pub unsafe trait Serialize: Sized {
    fn serialize(&self, buffer: &mut [u32]) -> usize {
        let len = buffer.len() * 4; // Convert to byte length
        let length = if len < size_of::<Self>() {
            len
        } else {
            size_of::<Self>()
        };

        unsafe {
            copy_nonoverlapping(transmute(self), buffer.as_mut_ptr(), length);
        }
        length
    }
}

unsafe impl Serialize for u8 {}
unsafe impl Serialize for u16 {}
unsafe impl Serialize for u32 {}
unsafe impl Serialize for u64 {}
unsafe impl Serialize for usize {}
unsafe impl Serialize for i8 {}
unsafe impl Serialize for i16 {}
unsafe impl Serialize for i32 {}
unsafe impl Serialize for i64 {}
unsafe impl Serialize for isize {}
