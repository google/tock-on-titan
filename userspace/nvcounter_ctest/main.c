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

#include <stdio.h>
#include <string.h>

int main(void) {
  printf("= Testing Nonvolatile Counter Driver =\n");
  for (int i = 0; i < 100; i++) {
    unsigned int val;
    int rval = tock_nvcounter_increment(&val);
    printf("Increment %i is %u\n", i, rval);
  }
  return 0;
}
