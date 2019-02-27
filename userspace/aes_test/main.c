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
#include "aes.h"

static unsigned char key[16] =    "1234567890123456";
static unsigned char data[16]   = "Data to encrypt.";
static unsigned char output[16];

int main(void) {
  printf("==== Starting Encryption ====\n");
  printf("Setting up key.\n");
  aes128_set_key_sync(key, strlen((char*)key));
  printf("Copying data %p to buffer %p.\n", data, output);
  memcpy(output, data, 16);
  printf("Encrypting %p: %s.\n", output, output);

  for (int i = 0; i < 16; i++) {
    printf("%02x", output[i]);
  }
  printf("\n");

  int rcode;

  rcode = aes128_encrypt_ecb(output, 16);
  if (rcode >= 0) {
    printf("Result    [%d]:\n", rcode);
    for (int i = 0; i < 16; i++) {
      printf("%02x", output[i]);
    }
    printf("\n");
  } else {
    printf("Error while encrypting: %d\n", -rcode);
    return -1;
  }
  printf("Decrypting %p\n", output);

  rcode = aes128_decrypt_ecb(output, 16);
  if (rcode >= 0) {
    printf("Result    [%d]:\n", rcode);
    for (int i = 0; i < 16; i++) {
      printf("%02x", output[i]);
    }
    printf("\n");
  } else {
    printf("Error while decrypting: %d\n", -rcode);
    return -1;
  }

}
