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

#include <tock.h>
#include "dcrypto_syscalls.h"

#define H1_DRIVER_DCRYPTO 0x40004

#define TOCK_DCRYPTO_CMD_CHECK 0
#define TOCK_DCRYPTO_CMD_RUN   1

#define TOCK_DCRYPTO_ALLOW_DATA 0
#define TOCK_DCRYPTO_ALLOW_PROG 1

#define TOCK_DCRYPTO_RUN_DONE 0

//#define TOCK_DCRYPTO_FAULT_STACK_OVERFLOW  2
//#define TOCK_DCRYPTO_FAULT_LOOP_OVERFLOW   3
//#define TOCK_DCRYPTO_FAULT_LOOP_UNDERFLOW  4
//#define TOCK_DCRYPTO_FAULT_DATA_ACCESS     5
//#define TOCK_DCRYPTO_FAULT_BREAK           7
//#define TOCK_DCRYPTO_FAULT_TRAP            8
//#define TOCK_DCRYPTO_FAULT_FAULT          10
//#define TOCK_DCRYPTO_FAULT_LOOP_MODRANGE  11
#define TOCK_DCRYPTO_FAULT_UNKNOWN        12

int last_error = 0;
int last_fault = 0;

const char* TOCK_DCRYPTO_ERRORS[] = {
                               "?",
                               "?",
                               "call stack overflow",
                               "loop stack underflow",
                               "loop stack overflow",
                               "data access",
                               "?",
                               "break",
                               "trap",
                               "?",
                               "fault",
                               "mod operand range",
                               "unknown"};

static const char* tock_dcrypto_fault_to_str(int fault) {
    if (fault <= TOCK_DCRYPTO_FAULT_UNKNOWN) {
      return TOCK_DCRYPTO_ERRORS[fault];
    } else {
      return "?";
    }
}

static void tock_dcrypto_run_done(int error,
                              int fault,
                              int unused __attribute__((unused)),
                              void *callback_args) {
  last_error = error;
  last_fault = fault;
  *(bool*)callback_args = true;
}

int tock_dcrypto_check(void) {
  return command(H1_DRIVER_DCRYPTO, TOCK_DCRYPTO_CMD_CHECK, 0, 0);
}

int tock_dcrypto_run(void* data, size_t datalen,
                     void* program, size_t programlen,
                     size_t start_instruction) {

  int ret = -1;
  bool run_done = false;

  ret = subscribe(H1_DRIVER_DCRYPTO, TOCK_DCRYPTO_RUN_DONE,
                  tock_dcrypto_run_done, &run_done);
  if (ret < 0) {
    printf("Could not register dcrypto callback with kernel: %d\n", ret);
    return ret;
  }

  ret = allow(H1_DRIVER_DCRYPTO, TOCK_DCRYPTO_ALLOW_DATA,
              data, datalen);
  if (ret < 0) {
    // This should only occur if application state is not available,
    // which means the driver is busy.
    printf("Could not give kernel access to dcrypto data: %d\n", ret);
    return TOCK_EBUSY;
  }

  ret = allow(H1_DRIVER_DCRYPTO, TOCK_DCRYPTO_ALLOW_PROG,
              program, programlen);
  if (ret < 0) {
    // This should only occur if application state is not available,
    // which means the driver is busy.
    printf("Could not give kernel access to dcrypto program: %d\n", ret);
    return TOCK_EBUSY;
  }

  ret = command(H1_DRIVER_DCRYPTO, TOCK_DCRYPTO_CMD_RUN, start_instruction, 0);

  if (ret < 0) {
    printf("Could not invoke dcrypto program instruction %i rcode: %d\n", start_instruction, ret);
    return ret;
  }

  yield_for(&run_done);

  if (last_error != 0) {
    printf("\nDCRYPTO failed: %s (%i).\n", tock_dcrypto_fault_to_str(last_fault), last_fault);
    return last_fault;
  } else {
    return 0;
  }
}
