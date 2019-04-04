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

#ifndef H1B_KL_SYSCALLS_H
#define H1B_KL_SYSCALLS_H

// Set the input buffer for a call to step.
int tock_h1b_kl_set_input(const uint32_t input[8]);

// Set the output buffer for a call to step.
int tock_h1b_kl_set_output(uint32_t output[8]);

// Invoke a step of the keyladder, for a particular "certificate"
int tock_h1b_kl_step(uint32_t cert);

// Return 1 if the driver is installed, 0 otherwise
int tock_h1b_kl_check(void);


#endif // H1B_KL_SYSCALLS_H
