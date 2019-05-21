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

#ifndef TOCK_DCRYPTO_H
#define TOCK_DCRYPTO_H

#include <stdlib.h>

int tock_dcrypto_check(void);

// Run the program pointed to by program with the data pointed to by
// data as data memory. The lengths are in bytes, but only whole
// 4-byte words are used: partial words are not used. For example,
// calling tock_dcrypto_run with a datalen of 11 will result in only 8
// bytes of data being copied in and out from dcrypto memory, while
// calling it with a datalen of 12 will result in 12 bytes being
// copied in/out.
//
// While the function does not accept partial words, it does not assume
// alignment: data and program do not have to be word-aligned.
int tock_dcrypto_run(void* data, size_t datalen,
                     void* program, size_t programlen,
                     size_t instruction);

#endif
