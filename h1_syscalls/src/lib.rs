// Copyright 2020 Google LLC
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

#![crate_name = "h1_syscalls"]
#![crate_type = "rlib"]
#![no_std]
#![feature(asm, core_intrinsics, const_fn, naked_functions)]

extern crate h1;
#[macro_use(static_init, debug)]
extern crate kernel;

pub mod digest;
pub mod aes;
pub mod dcrypto;
pub mod dcrypto_test;
pub mod debug_syscall;
pub mod nvcounter_syscall;
pub mod personality;

pub unsafe fn init() {
}
