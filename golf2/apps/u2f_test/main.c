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

#include "u2f.h"

static char u2f_received_frame[64];

int main(void) {
  int ret = 0;

  delay_ms(2000);
  printf("= Running U2F Transport Test =\n");

  while (1) {
    printf("1. Receiving a U2F frame over USB.\n");
    ret = tock_u2f_receive(u2f_received_frame, 64);
    printf("   Received with code %i", ret);
    for (int i = 0; i < 64; i++) {
      if (i % 32 == 0) {
        printf("\n");
      }
      printf("%02x ", u2f_received_frame[i]);
    }
    printf("\n");
  }
}
