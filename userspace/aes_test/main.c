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

#include <gpio.h>
#include <stdio.h>
#include <string.h>

#include "h1b_aes_syscalls.h"

static unsigned char key[] = "1234567890123456";
static unsigned char data[] = "Data to encrypt. We shall see if this works.....";
static unsigned char buffer[48];

void print_buffer(unsigned char *buffer, size_t length);

void print_buffer(unsigned char *buffer, size_t length) {
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
  tock_aes_set_key(key, strlen((const char*)key));
  printf("Encrypting %i bytes.\n", strlen((const char*)data));
  memcpy(buffer, data, strlen((const char*)data) + 1);
  int len = tock_aes_encrypt_ecb_sync(16, buffer, sizeof(buffer));

  if (len >= 0) {
    printf("Result    [%i]: ", len);
    print_buffer(buffer, sizeof(buffer));
  } else {
    printf("Got error while encrypting: %d\n", -len);
    return -1;
  }

  printf("\n");
  printf("==== Starting Decryption ====\n");

  printf("Expecting [%d]: ", strlen((const char*)data));
  print_buffer(data, strlen((const char*)data));

  int res;
  printf("Setting up key.\n");
  res = tock_aes_set_key(key, strlen((const char*)key));
  if (res < 0) {
    printf("Got error while setup: %d\n", res);
  }
  printf("Decrypting.\n");
  int dec_len = tock_aes_decrypt_ecb_sync(16, buffer, sizeof(buffer));

  printf("Result    [%d]: ", dec_len);
  print_buffer(buffer, sizeof(buffer));

}
