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

#include "nvcounter_syscalls.h"
#include "tock.h"

#define H1_DRIVER_NVCOUNTER 0x80040000

#define TOCK_NVCOUNTER_CMD_CHECK   0
#define TOCK_NVCOUNTER_CMD_INCREMENT     1

#define TOCK_NVCOUNTER_INCREMENT_DONE    0

// We store the pointer to where we should store
// the updated counter as a global. This is protected
// by a successful call to the command, so it's only
// overwritten if the command is successful. Furthermore,
// after a callback it's reset to NULL.
static unsigned int* counter_global = NULL;

static void tock_nvcounter_increment_done(int code __attribute__ ((unused)),
                                          int counter,
                                          int unused2 __attribute__((unused)),
                                          void *callback_args) {
  *(bool*)callback_args = true;
  if (counter_global != NULL) {

    *counter_global = (unsigned int)counter;
    counter_global = NULL;
  }
}

int tock_nvcounter_check(void) {
  return command(H1_DRIVER_NVCOUNTER, TOCK_NVCOUNTER_CMD_CHECK, 0, 0);
}

int tock_nvcounter_increment(unsigned int* counter) {
  int ret = 0;
  bool increment_done = false;

  ret = subscribe(H1_DRIVER_NVCOUNTER, TOCK_NVCOUNTER_INCREMENT_DONE,
                  tock_nvcounter_increment_done, &increment_done);
  if (ret < 0) {
    printf("Could not register for NV counter increment callback.\n");
    return ret;
  }

  ret = command(H1_DRIVER_NVCOUNTER, TOCK_NVCOUNTER_CMD_INCREMENT,
                0, 0);
  if (ret < 0) {
    printf("Could not increment NV counter: %s (%i).\n", tock_strerror(ret), ret);
    return ret;
  }

  counter_global = counter;
  yield_for(&increment_done);

  return TOCK_SUCCESS;
}
