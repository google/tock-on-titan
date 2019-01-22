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


static unsigned char key[16] =    "1234567890123456";
static unsigned char data[16]   = "Data to encrypt.";
static unsigned char output[16];
static char expected[] = {
    0x25, 0x97, 0xea, 0xce, 0x3a, 0x51, 0xce, 0x0d, 0xd8, 0x97, 0xae, 0x00,
    0x2a, 0x4e, 0xcd, 0xac, 0xe6, 0x31, 0xf6, 0x42, 0xa3, 0xbe, 0x5f, 0xaf,
    0x3e, 0x8f, 0x04, 0x39, 0xf5, 0x9c, 0x40, 0x96, 0x6f, 0x21, 0x4b, 0xce,
    0x1a, 0x1f, 0xbc, 0xdf, 0x95, 0x26, 0xc4, 0xf1, 0xff, 0xed, 0xf1, 0x22};

//static char output[8000 / 8];
//static char decrypted[8000 / 8];

void print_buffer(char *buffer, size_t length, const char *format);

void print_buffer(char *buffer, size_t length, const char *format) {
  for (size_t i = 0; i < length; i++) {
    printf(format, buffer[i]);
    fflush(stdout);
  }
  printf("\n");
}

int main(void) {
  printf("==== Starting Encryption ====\n");
  printf("Setting up key.\n");
  aes128_set_key_sync(key, strlen(key));
  printf("Copying data %p to buffer %p.\n", data, output);
  memcpy(output, data, 16);
  printf("Encrypting %p: %s.\n", output, output);
  print_buffer(output, 16, "%02x ");

  int rcode;

  rcode = aes128_encrypt_ecb(output, 16);
  if (rcode >= 0) {
    printf("Result    [%d]:\n", rcode);
    print_buffer(output, 16, "%02x ");
  } else {
    printf("Error while encrypting: %d\n", -rcode);
    return -1;
  }
  printf("Decrypting %p\n", output);

  rcode = aes128_decrypt_ecb(output, 16);
  if (rcode >= 0) {
    printf("Result    [%d]:\n", rcode);
    print_buffer(output, 16, "%02x ");
  } else {
    printf("Error while decrypting: %d\n", -rcode);
    return -1;
  }

}
