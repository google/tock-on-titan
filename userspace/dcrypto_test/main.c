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
/* static char key[] = "1234567890123456";
static char expected[] = {
    0x25, 0x97, 0xea, 0xce, 0x3a, 0x51, 0xce, 0x0d, 0xd8, 0x97, 0xae, 0x00,
    0x2a, 0x4e, 0xcd, 0xac, 0xe6, 0x31, 0xf6, 0x42, 0xa3, 0xbe, 0x5f, 0xaf,
    0x3e, 0x8f, 0x04, 0x39, 0xf5, 0x9c, 0x40, 0x96, 0x6f, 0x21, 0x4b, 0xce,
    0x1a, 0x1f, 0xbc, 0xdf, 0x95, 0x26, 0xc4, 0xf1, 0xff, 0xed, 0xf1, 0x22};

static char output[8000 / 8];
static char decrypted[8000 / 8];*/



void print_buffer(char *buffer, size_t length, const char *format);

void print_buffer(char *buffer, size_t length, const char *format) {
  for (size_t i = 0; i < length; i++) {
    printf(format, buffer[i]);
    fflush(stdout);
  }
  printf("\n");
}

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
  //printf("Return value: %i.\n", ret);
  //printf("\n");

  /*
  printf("Expecting [%d]: 0x", sizeof(expected));
  print_buffer(expected, sizeof(expected), "%02x");
  printf("Setting up key.\n");
  tock_aes_setup(key, strlen(key), TOCK_AES_SIZE_128, TOCK_AES_ENCRYPT);
  printf("Encrypting.\n");
  int len = tock_aes_crypt(data, strlen(data), output, sizeof(output));
  tock_aes_finish();

  if (len >= 0) {
    printf("Result    [%d]: 0x", len);
    print_buffer(output, len, "%02x");
  } else {
    printf("Got error while encrypting: %d\n", -len);
    return -1;
  }
  
  printf("\n");
  printf("==== Starting Decryption ====\n");

  printf("Expecting [%d]: ", sizeof(data));
  print_buffer(data, strlen(data), "%c");

  int res;
  printf("Setting up key.\n");
  res = tock_aes_setup(key, strlen(key), TOCK_AES_SIZE_128, TOCK_AES_DECRYPT);
  if (res < 0) {
    printf("Got error while setup: %d\n", res);
  }
  printf("Decrypting.\n");
  int dec_len = tock_aes_crypt(output, len, decrypted, sizeof(decrypted));
  tock_aes_finish();

  if (dec_len >= 0) {
    printf("Result    [%d]: ", dec_len);
    print_buffer(decrypted, dec_len, "%c");
  } else {
    printf("Got error while decrypting: %d\n", -dec_len);
    return -1;
  }
  */
}
