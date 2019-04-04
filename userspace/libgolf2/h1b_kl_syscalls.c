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

#include "tock.h"
#include "h1b_kl_syscalls.h"

static const uint32_t* input_ptr = NULL;
static uint32_t* output_ptr = NULL;

#define H1B_KL_DRIVER 0x40005

#define H1B_KL_CMD_CHECK 0
#define H1B_KL_CMD_STEP  1

#define H1B_KL_ALLOW_INPUT  0
#define H1B_KL_ALLOW_OUTPUT 1

// Set the input buffer for a call to step.
int tock_h1b_kl_set_input(const uint32_t input[8]) {
  // Because a very common call pattern is to invoke step many times
  // with the same arguments (e.g., NULL, NULL), insert this check
  // to reduce common case syscall count from 3 to 1.
  if (input != input_ptr) {
    input_ptr = input;
    return allow(H1B_KL_DRIVER, H1B_KL_ALLOW_INPUT, (void*)input, 32);
  } else {
    return TOCK_SUCCESS;
  }
}

// Set the output buffer for a call to step.
int tock_h1b_kl_set_output(uint32_t output[8]) {
  // Because a very common call pattern is to invoke step many times
  // with the same arguments (e.g., NULL, NULL), insert this check
  // to reduce common case syscall count from 3 to 1.
  if (output != output_ptr) {
    output_ptr = output;
    return allow(H1B_KL_DRIVER, H1B_KL_ALLOW_OUTPUT, (void*)output, 32);
  } else {
    return TOCK_SUCCESS;
  }
}

// Invoke a step of the keyladder, for a particular "certificate"
int tock_h1b_kl_step(uint32_t cert) {
  return command(H1B_KL_DRIVER, H1B_KL_CMD_STEP, cert, 0);
}

int tock_h1b_kl_check(void) {
  return command(H1B_KL_DRIVER, H1B_KL_CMD_CHECK, 0, 0);
}
