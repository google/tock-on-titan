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

// This code is a rewrite/port of the u2f_transport.c code

#include <stdio.h>
#include <string.h>
#include <timer.h>
#include <rng.h>

#include "u2f.h"
#define FLASH_ENTROPY_SIZE 0x200

#define P256_BITSPERDIGIT 32
#define P256_NDIGITS 8
#define P256_NBYTES 32
#define P256_DIGIT(x, y) ((x)->a[y])

typedef uint32_t p256_digit;
typedef uint64_t p256_ddigit;
typedef int64_t p256_sddigit;

typedef struct {
  p256_digit a[P256_NDIGITS];
} p256_int;

/* individual attestation data */
typedef struct {
  uint32_t chksum[8];
  uint32_t salt[8];
  p256_int pub_x;
  p256_int pub_y;
  uint32_t cert_hash[8];
  size_t cert_len;
  uint8_t cert[2048 - 4 - 5 * 32];
} perso_st;

char fips_entropy[FLASH_ENTROPY_SIZE];

static char u2f_command_frame [] = {0x00, 0x00, 0x00, 0xaa, // Channel ID
                                    0x80 | 0x3f, // Command: U2F error
                                    0x00, // bcount high
                                    0x01, // bcount low
                                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // 8-15
                                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // 16-23
                                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // 24-31
                                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // 32-39
                                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // 40-47
                                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // 48-55
                                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00}; // 56-63

static char u2f_received_frame[64];

const perso_st* get_personality() {return NULL;}
int check_personality(const perso_st* id) {return 1;}
int new_personality(perso_st* id) {return 0;}
int set_personality(const perso_st* id) {return 0;}

// THIS IS NOT SHA256 -- just a placeholder
void sha256(uint8_t* input, uint32_t input_len, uint32_t* output) {
  for (uint32_t i = 0; i < input_len; i++) {
    uint32_t index = (i / 4) % 8;
    uint32_t offset = i % 4;
    output[index] = output[index] ^ ((uint32_t)input) << (offset * 8);
  }
}

// Make sure there's entropy. Should only generate on first boot, then store
// in flash. Until flash driver is ready, just store in RAM.
void ensure_factory_entropy() {
  uint32_t ones = -1u, v;
  uint8_t entropy[128];  // 1024 bits
  uint32_t digest[8];    // SHA256 digest
  for (int i = 0; i < FLASH_ENTROPY_SIZE; i += sizeof(digest)) {
    rng_sync(entropy, sizeof(entropy), sizeof(entropy));
    sha256(entropy, sizeof(entropy), digest);
    memcpy(fips_entropy + i, digest, sizeof(digest));
  }
  printf("    - Entropy generated:");
  for (int i = 0; i < FLASH_ENTROPY_SIZE / 64 ; i++) {
    if (i % 4 == 0) {
      printf(" ");
    }
    printf("%02x", fips_entropy[i]);
  }
  printf("\n");
}

void setup_personality() {
  perso_st me;
  if (check_personality(get_personality()) == 1) return;
  if (new_personality(&me) == 1) set_personality(&me);
  printf("    - Personality configured\n");
}


void check_device_setup() {
  perso_st me;
  printf("  - Checking setup\n");
  ensure_factory_entropy();
  setup_personality();
}



int main(void) {
  int ret = 0;

  delay_ms(2000);
  printf("= Running U2F Transport Application =\n");
  delay_ms(100);
  check_device_setup();

  while (1) {
    printf("1. Receiving a U2F frame over USB.\n");
    ret = tock_u2f_receive(u2f_received_frame, 64);
    printf("   Received");
    for (int i = 0; i < 64; i++) {
      if (i % 32 == 0) {
        printf("\n");
      }
      printf("%02x ", u2f_received_frame[i]);
    }
    printf("\n");
    //printf("1. Transmitting a U2F error packet over transport.\n");
    //ret = tock_u2f_transmit(u2f_command_frame, 64);
    //printf("Return value: %i.\n", ret);
  }
}
