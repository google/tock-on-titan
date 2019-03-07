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


#include "aes_ecb_syscalls.h"

#define AES_DRIVER 0x40000

#define TOCK_AES_CMD_CHECK 0
#define TOCK_AES_CMD_ECB_ENC 1
#define TOCK_AES_CMD_ECB_DEC 2
#define TOCK_AES_CMD_CTR_ENC 3
#define TOCK_AES_CMD_CTR_DEC 4
#define TOCK_AES_CMD_CBC_ENC 5
#define TOCK_AES_CMD_CBC_DEC 6

#define TOCK_AES_ALLOW_KEY    0
#define TOCK_AES_ALLOW_INPUT  1
#define TOCK_AES_ALLOW_OUTPUT 2
#define TOCK_AES_ALLOW_IVCTR  3

#define TOCK_AES_SUBSCRIBE_CRYPT 0

// Struct used for creating synchronous versions of functions.
//
// fired - set when the callback has been called
// error - error received from the kernel, less than zero indicates an error
typedef struct {
  bool fired;
  int error;
} aes_data_t;

// Internal callback for creating synchronous functions
//
// callback_type - number indicating which type of callback occurred
// currently 1(encryption) and 2(decryption)
// callback_args - user data passed into the set_callback function
static void aes_cb(int callback_type,
                   __attribute__ ((unused)) int unused1,
                   __attribute__ ((unused)) int unused2,
                   void *callback_args) {

  aes_data_t *result = (aes_data_t*)callback_args;
  result->fired = true;
  result->error = callback_type;
}

int tock_aes128_set_callback(subscribe_cb callback, void *ud);

// ***** System Call Interface *****

// Internal callback for encryption and decryption
static int tock_aes128_set_callback(subscribe_cb callback, void *ud) {
  return subscribe(AES_DRIVER, TOCK_AES_SUBSCRIBE_CRYPT, callback, ud);
}


int tock_aes128_set_input(unsigned char *data, unsigned char len) {
  return allow(AES_DRIVER, TOCK_AES_ALLOW_INPUT, (void*)data, len);
}

int tock_aes128_set_key(const unsigned char* data, unsigned char len) {
  return allow(AES_DRIVER, TOCK_AES_ALLOW_KEY, (void*)data, len);
}

int tock_aes128_set_output(unsigned char* data, unsigned char len) {
  return allow(AES_DRIVER, TOCK_AES_ALLOW_OUTPUT, (void*)data, len);
}

// Internal function to configure a initial counter to be used on counter-mode
static int tock_aes128_set_ctr(unsigned char* ctr, unsigned char len) {
  return allow(AES_DRIVER, TOCK_AES_ALLOW_IVCTR, (void*)ctr, len);
}

// ***** Synchronous Calls *****


// Function to encrypt by aes128 counter-mode with a given payload and
// initial counter synchronously
int tock_aes128_encrypt_ctr_sync(unsigned char* buf, unsigned char buf_len,
                                 unsigned char* ctr, unsigned char ctr_len) {

  int err;
  aes_data_t result = { .fired = false, .error = TOCK_SUCCESS };

  err = tock_aes128_set_callback(aes_cb, &result);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_input(buf, buf_len);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_ctr(ctr, ctr_len);
  if (err < TOCK_SUCCESS) return err;

  err = command(AES_DRIVER, TOCK_AES_CMD_CTR_ENC, 0, 0);
  if (err < TOCK_SUCCESS) return err;

  yield_for(&result.fired);

  return result.error;
}


// Function to decrypt by tock_aes128 counter-mode with a given payload and
// initial counter synchronously
int tock_aes128_decrypt_ctr_sync(unsigned char* buf, unsigned char buf_len,
                                 unsigned char* ctr, unsigned char ctr_len) {

  int err;
  aes_data_t result = { .fired = false, .error = TOCK_SUCCESS };

  err = tock_aes128_set_callback(aes_cb, &result);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_input(buf, buf_len);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_ctr(ctr, ctr_len);
  if (err < TOCK_SUCCESS) return err;

  err = command(AES_DRIVER, TOCK_AES_CMD_CTR_DEC, 0, 0);
  if (err < TOCK_SUCCESS) return err;

  yield_for(&result.fired);

  return result.error;
}


int tock_aes128_encrypt_ecb_sync(unsigned char* buf, unsigned char buf_len) {
  int err;
  aes_data_t result = { .fired = false, .error = TOCK_SUCCESS };

  err = tock_aes128_set_callback(aes_cb, &result);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_input(buf, buf_len);
  if (err < TOCK_SUCCESS) return err;

  err = command(AES_DRIVER, TOCK_AES_CMD_ECB_ENC, 0, 0);
  if (err < TOCK_SUCCESS) return err;

  yield_for(&result.fired);

  return result.error;
}

int tock_aes128_decrypt_ecb_sync(unsigned char* buf, unsigned char buf_len) {
  int err;
  aes_data_t result = { .fired = false, .error = TOCK_SUCCESS };

  err = tock_aes128_set_callback(aes_cb, &result);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_input(buf, buf_len);
  if (err < TOCK_SUCCESS) return err;

  err = command(AES_DRIVER, TOCK_AES_CMD_ECB_DEC, 0, 0);
  if (err < TOCK_SUCCESS) return err;

  yield_for(&result.fired);

  return result.error;
}

int tock_aes128_encrypt_cbc_sync(unsigned char* buf, unsigned char buf_len,
                                 unsigned char* iv, unsigned char iv_len) {
  int err;
  aes_data_t result = { .fired = false, .error = TOCK_SUCCESS };

  err = tock_aes128_set_callback(aes_cb, &result);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_input(buf, buf_len);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_ctr(iv, iv_len);
  if (err < TOCK_SUCCESS) return err;

  err = command(AES_DRIVER, TOCK_AES_CMD_CBC_ENC, 0, 0);
  if (err < TOCK_SUCCESS) return err;

  yield_for(&result.fired);

  return result.error;
}

int tock_aes128_decrypt_cbc_sync(unsigned char* buf, unsigned char buf_len,
                                 unsigned char* iv, unsigned char iv_len) {
  int err;
  aes_data_t result = { .fired = false, .error = TOCK_SUCCESS };

  err = tock_aes128_set_callback(aes_cb, &result);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_input(buf, buf_len);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_ctr(iv, iv_len);
  if (err < TOCK_SUCCESS) return err;

  err = command(AES_DRIVER, TOCK_AES_CMD_CBC_DEC, 0, 0);
  if (err < TOCK_SUCCESS) return err;

  yield_for(&result.fired);

  return result.error;
}
