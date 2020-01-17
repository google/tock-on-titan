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

#include <stdint.h>
#include <stdio.h>
#include <tock.h>
#include <rng.h>

struct rng_data {
  bool done;
  int len;
};

int main(void) {
  printf("Boooting RNG test application. Should output 250 words.\n");

  unsigned long buf[250];
  int size = sizeof(buf);
  int len = rng_sync((uint8_t*)buf, size, size);
  len = len / 4;
  printf("Read %d words of random data.\n", len);
  for (int i = 0; i < len; i++) {
    printf("Sample data %i = 0x%08lx\n", i, buf[i]);
  }
  return 0;
}
