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

#include <stdio.h>
#include <string.h>
#include <timer.h>

#include "dcrypto.h"

static char program_return[] = {0x00, 0x00, 0x00, 0x0c}; // RET
static char program_recursion[] = {0x00, 0x00, 0x00, 0x08,  // CALL 0
                                   0x00, 0x00, 0x00, 0x00}; // BREAK

static char data[] = "Data to encrypt. We shall see if this works.";

int main(void) {
  int ret = 0;

  printf("==== Running DCRYPTO ====\n");

  printf("1. Testing simple return program: should succeed.\n");
  ret = tock_dcrypto_run(data, 10, program_return, 4);
  printf("Return value: %i.\n", ret);
  printf("\n");
  delay_ms(1000);

  printf("2. Testing infinite recursion: should overflow.\n");
  ret = tock_dcrypto_run(data, 10, program_recursion, 8);

}
