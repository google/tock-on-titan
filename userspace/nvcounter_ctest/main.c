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

// This tests the the nonvolatile counter userspace library.

#include "nvcounter_syscalls.h"
#include "timer.h"

#include <stdio.h>
#include <string.h>

unsigned int val;
int main(void) {
  printf("= Testing Nonvolatile Counter Driver =\n");
  int test = tock_nvcounter_check();
  if (test != TOCK_SUCCESS) {
    printf("ERROR: no Nonvolatile Counter syscall driver installed.");
  }
  for (int i = 0; i < 5; i++) {
    int rval = tock_nvcounter_increment(&val);
    printf("Increment %i is %u\n", i, val);
    delay_ms(1000);
  }
  return 0;
}
