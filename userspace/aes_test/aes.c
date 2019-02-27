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
#include "tock.h"
#include "aes.h"
#include "aes_ecb_syscalls.h"
int bytes_encrypted = 0;

static void tock_aes_encrypt_done_cb(int count,
                                    int unused2 __attribute__((unused)),
                                    int unused3 __attribute__((unused)),
                                    void *callback_args);

static void tock_aes_encrypt_done_cb(int count,
                                    int unused2 __attribute__((unused)),
                                    int unused3 __attribute__((unused)),
                                    void *callback_args) {
  bytes_encrypted = count;
  *(bool *)callback_args = true;
}

// function called by the encryption or decryption operation when they are
// finished
//
// callback       - pointer to function to be called
// callback_args  - pointer to data provided to the callback
int aes128_set_callback(subscribe_cb callback, void *callback_args) {
  return subscribe(AES_DRIVER, TOCK_AES_SUBSCRIBE_CRYPT, callback, callback_args);
}


// configures a buffer with data to be used for encryption or decryption
//
// data           - buffer with data
// len            - length of the data buffer
int aes128_set_data(const unsigned char *data, unsigned char len) {
  return allow(AES_DRIVER, TOCK_AES_ALLOW_INPUT, (unsigned char*)data, len);
}


// configures an encryption key to be used for encryption and decryption
//
// key - a buffer containing the key (should be 16 bytes for aes128)
// len - length of the buffer (should be 16 bytes for aes128)
int aes128_set_key_sync(const unsigned char* key, unsigned char len) {
  int rval = allow(AES_DRIVER, TOCK_AES_ALLOW_KEY, (unsigned char*)key, len);
  if (rval != TOCK_SUCCESS) {
    printf("Failed to install AES128 key: %i\n", rval);
  }
  return rval;
}

int aes128_encrypt_ecb(unsigned char* buf, unsigned char buf_len) {
  bool aes = false;

  int rval = subscribe(AES_DRIVER, TOCK_AES_SUBSCRIBE_CRYPT, &tock_aes_encrypt_done_cb, &aes);
  if (rval != TOCK_SUCCESS) {
    printf("Failed to register callback for AES128 ECB encryption: %i\n", rval);
    return rval;
  }

  rval = aes128_set_data(buf, buf_len);
  if (rval != TOCK_SUCCESS) {
    printf("Failed to install buffer for AES128 ECB encryption: %i\n", rval);
    return rval;
  }

  rval = command(AES_DRIVER, TOCK_AES_CMD_ECB_ENC, 0, 0);
  if (rval != TOCK_SUCCESS) {
    printf("Failed to invoke AES128 ECB encryption: %i\n", rval);
    return rval;
  }

  yield_for(&aes);

  return TOCK_SUCCESS;
}

int aes128_decrypt_ecb(unsigned char* buf, unsigned char buf_len) {
  bool aes = false;

  int rval = subscribe(AES_DRIVER, TOCK_AES_SUBSCRIBE_CRYPT, &tock_aes_encrypt_done_cb, &aes);
  if (rval != TOCK_SUCCESS) {
    printf("Failed to register callback for AES128 ECB decryption: %i\n", rval);
    return rval;
  }

  rval = aes128_set_data(buf, buf_len);
  if (rval != TOCK_SUCCESS) {
    printf("Failed to install buffer for AES128 ECB decryption: %i\n", rval);
    return rval;
  }

  rval = command(AES_DRIVER, TOCK_AES_CMD_ECB_DEC, 0, 0);
  if (rval != TOCK_SUCCESS) {
    printf("Failed to invoke AES128 ECB decryption: %i\n", rval);
    return rval;
  }

  yield_for(&aes);

  return TOCK_SUCCESS;
}
