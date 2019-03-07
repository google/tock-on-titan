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

#include <gpio.h>
#include <stdio.h>
#include <string.h>

#include "aes_ecb_syscalls.h"

static unsigned char key[] = "1234567890123456";
static unsigned char data[] = "Data to encrypt. We shall see if this works.....";
static unsigned char expected[] = {
    0x25, 0x97, 0xea, 0xce, 0x3a, 0x51, 0xce, 0x0d, 0xd8, 0x97, 0xae, 0x00,
    0x2a, 0x4e, 0xcd, 0xac, 0xe6, 0x31, 0xf6, 0x42, 0xa3, 0xbe, 0x5f, 0xaf,
    0x3e, 0x8f, 0x04, 0x39, 0xf5, 0x9c, 0x40, 0x96, 0x6f, 0x21, 0x4b, 0xce,
    0x1a, 0x1f, 0xbc, 0xdf, 0x95, 0x26, 0xc4, 0xf1, 0xff, 0xed, 0xf1, 0x22};

static char buffer[48];

void print_buffer(char *buffer, size_t length);

void print_buffer(char *buffer, size_t length) {
  for (size_t i = 0; i < length; i++) {
    printf("%02x ", buffer[i]);
    fflush(stdout);
  }
  printf("\n");
}

int main(void) {
  printf("==== Starting Encryption ====\n");
  //  printf("Expecting [%d]: 0x", sizeof(expected));
  // print_buffer(expected, sizeof(expected), "%02x");
  printf("Setting up key.\n");
  tock_aes128_set_key(key, strlen(key));
  printf("Encrypting %i bytes.\n", strlen(data));
  memcpy(buffer, data, strlen(data) + 1);
  int len = tock_aes128_encrypt_ecb_sync(buffer, sizeof(buffer));

  if (len >= 0) {
    printf("Result    [%d]: 0x", len);
    print_buffer(buffer, sizeof(buffer));
  } else {
    printf("Got error while encrypting: %d\n", -len);
    return -1;
  }

  printf("\n");
  printf("==== Starting Decryption ====\n");

  printf("Expecting [%d]: ", sizeof(data));
  print_buffer(data, strlen(data));

  int res;
  printf("Setting up key.\n");
  res = tock_aes128_set_key(key, strlen(key));
  if (res < 0) {
    printf("Got error while setup: %d\n", res);
  }
  printf("Decrypting.\n");
  int dec_len = tock_aes128_decrypt_ecb_sync(buffer, sizeof(buffer));

  printf("Result    [%d]: ", dec_len);
  print_buffer(buffer, sizeof(buffer));

}
