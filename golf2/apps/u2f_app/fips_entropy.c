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

#include "fips.h"
#include "storage.h"

// libtock-c
#include "rng.h"

#define FLASH_ENTROPY_SIZE FLASH_INFO_MANUFACTURE_STATE_SIZE

static char fips_entropy[FLASH_ENTROPY_SIZE];
static uint32_t counter = 0;

uint32_t flash_ctr_incr(void) {
  counter++;
  return counter;
}

int flash_info_read_enable(uint32_t addr __attribute__((unused)),
                           uint32_t len __attribute__((unused))) {return 0;}
int flash_info_read_disable(void) {return 0;}

uint32_t flash_physical_info_read_word(uint32_t addr, uint32_t* dest) {
  uint32_t* words = (uint32_t*)fips_entropy;
  *dest = words[addr];
  return 0;
}

// Make sure there's entropy. Should only generate on first boot, then store
// in flash. Until flash driver is ready, just store in RAM.
void ensure_factory_entropy(void) {
  //uint32_t ones = -1u, v;
  uint8_t entropy[128];  // 1024 bits
  uint32_t digest[8];    // SHA256 digest
  printf("Generating entropy:");
  for (int i = 0; i < FLASH_ENTROPY_SIZE; i += (8 * sizeof(uint32_t))) {
    rng_sync(entropy, sizeof(entropy), sizeof(entropy));
    SHA256(entropy, sizeof(entropy), (uint8_t*)digest);
    memcpy(fips_entropy + i, digest, sizeof(digest));
  }
  for (int i = 0; i < FLASH_ENTROPY_SIZE; i++) {
    if (i % 32 == 0) {
      printf("\n  ");
    } else if (i % 4 == 0) {
      printf(" ");
    }
    printf("%02x", fips_entropy[i]);
  }
  printf("\n\n");
}
