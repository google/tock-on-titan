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


#include "h1b_aes_syscalls.h"

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
//#define TOCK_AES_ALLOW_OUTPUT 2
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

static int tock_aes128_set_callback(subscribe_cb callback, void *ud);
static int tock_aes128_set_input(unsigned char *data, unsigned char len);

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

// ***** System Call Interface *****

int tock_aes128_check(void) {
  return command(AES_DRIVER, TOCK_AES_CMD_CHECK, 0, 0);
}

// Internal callback for encryption and decryption
static int tock_aes128_set_callback(subscribe_cb callback, void *ud) {
  return subscribe(AES_DRIVER, TOCK_AES_SUBSCRIBE_CRYPT, callback, ud);
}

static int tock_aes128_set_input(unsigned char *data, unsigned char len) {
  return allow(AES_DRIVER, TOCK_AES_ALLOW_INPUT, (void*)data, len);
}

// Internal function to configure a initial counter to be used on counter-mode
static int tock_aes128_set_ctr(unsigned char* ctr, unsigned char len) {
  return allow(AES_DRIVER, TOCK_AES_ALLOW_IVCTR, (void*)ctr, len);
}

static void increment_counter(unsigned char* buf) {
  // Start from least significant byte (big endian), carry left
  for (int i = 15; i >= 0; i--) {
    unsigned char c = buf[i] + 1;
    buf[i] = c;
    if (c != 0) {
      // If it's 0 we overflowed - carry to next byte
      return;
    }
  }
}


// ***** Synchronous Calls *****


int tock_aes128_set_key(const unsigned char* data, unsigned char len) {
  return allow(AES_DRIVER, TOCK_AES_ALLOW_KEY, (void*)data, len);
}

// Operates on a single 16-byte block.
// buf and ctr are assumed to be >= 16 bytes.
static int aes128_encrypt_ctr_block(unsigned char* buf, unsigned char* ctr) {
  int err;
  aes_data_t result = { .fired = false, .error = TOCK_SUCCESS };

  err = tock_aes128_set_callback(aes_cb, &result);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_input(buf, 16);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_ctr(ctr, 16);
  if (err < TOCK_SUCCESS) return err;

  err = command(AES_DRIVER, TOCK_AES_CMD_CTR_ENC, 0, 0);
  if (err < TOCK_SUCCESS) return err;

  yield_for(&result.fired);

  if (result.error == TOCK_SUCCESS) {
    increment_counter(ctr);
  }

  return result.error;
}

// Operates on a single 16-byte block.
// buf and ctr are assumed to be >= 16 bytes.
static int aes128_decrypt_ctr_block(unsigned char* buf, unsigned char* ctr) {
  int err;
  aes_data_t result = { .fired = false, .error = TOCK_SUCCESS };

  err = tock_aes128_set_callback(aes_cb, &result);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_input(buf, 16);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_ctr(ctr, 16);
  if (err < TOCK_SUCCESS) return err;

  err = command(AES_DRIVER, TOCK_AES_CMD_CTR_DEC, 0, 0);
  if (err < TOCK_SUCCESS) return err;

  yield_for(&result.fired);

  if (result.error == TOCK_SUCCESS) {
    increment_counter(ctr);
  }

  return result.error;
}

int tock_aes128_encrypt_ctr_sync(unsigned char* buf, unsigned char buf_len,
                                 unsigned char* ctr, unsigned char ctr_len) {
  if ((buf_len % 16 != 0) || (ctr_len != 16)) {
    return TOCK_ESIZE;
  }

  // Just encrypt each block
  int err = TOCK_SUCCESS;
  for (int i = 0; i < buf_len; i+= 16) {
    err = aes128_encrypt_ctr_block(buf + i, ctr);
    if (err != TOCK_SUCCESS){
      return err;
    }
  }
  return err;
}

int tock_aes128_decrypt_ctr_sync(unsigned char* buf, unsigned char buf_len,
                                 unsigned char* ctr, unsigned char ctr_len) {
  if ((buf_len % 16 != 0) || (ctr_len != 16)) {
    return TOCK_ESIZE;
  }

  // Just decrypt each block
  int err = TOCK_SUCCESS;
  for (int i = 0; i < buf_len; i+= 16) {
    err = aes128_decrypt_ctr_block(buf + i, ctr);
    if (err != TOCK_SUCCESS){
      return err;
    }
  }
  return err;
}

// Assumes buf is 16 bytes long.
static int aes128_encrypt_ecb_block(unsigned char* buf) {
  int err;
  aes_data_t result = { .fired = false, .error = TOCK_SUCCESS };

  err = tock_aes128_set_callback(aes_cb, &result);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_input(buf, 16);
  if (err < TOCK_SUCCESS) return err;

  err = command(AES_DRIVER, TOCK_AES_CMD_ECB_ENC, 0, 0);
  if (err < TOCK_SUCCESS) return err;

  yield_for(&result.fired);

  return result.error;
}

// Assumes buf is 16 bytes long
static int aes128_decrypt_ecb_block(unsigned char* buf) {
  int err;
  aes_data_t result = { .fired = false, .error = TOCK_SUCCESS };

  err = tock_aes128_set_callback(aes_cb, &result);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_input(buf, 16);
  if (err < TOCK_SUCCESS) return err;

  err = command(AES_DRIVER, TOCK_AES_CMD_ECB_DEC, 0, 0);
  if (err < TOCK_SUCCESS) return err;

  yield_for(&result.fired);

  return result.error;
}

int tock_aes128_encrypt_ecb_sync(unsigned char* buf, unsigned char buf_len) {
  if (buf_len % 16 != 0) {
    return TOCK_ESIZE;
  }

  // Just encrypt each block
  int err = TOCK_SUCCESS;
  for (int i = 0; i < buf_len; i+= 16) {
    err = aes128_encrypt_ecb_block(buf + i);
    if (err != TOCK_SUCCESS){
      return err;
    }
  }
  return err;
}

int tock_aes128_decrypt_ecb_sync(unsigned char* buf, unsigned char buf_len) {
  if (buf_len % 16 != 0) {
    return TOCK_ESIZE;
  }

  // Just decrypt each block
  int err = TOCK_SUCCESS;
  for (int i = 0; i < buf_len; i+= 16) {
    err = aes128_decrypt_ecb_block(buf + i);
    if (err != TOCK_SUCCESS){
      return err;
    }
  }
  return err;
}

// Encrypt a block using CBC mode. Assumes both buf and iv are
// 16 bytes long.
static int aes128_encrypt_cbc_block(unsigned char* buf, unsigned char* iv) {
  int err;
  aes_data_t result = { .fired = false, .error = TOCK_SUCCESS };

  err = tock_aes128_set_callback(aes_cb, &result);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_input(buf, 16);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_ctr(iv, 16);
  if (err < TOCK_SUCCESS) return err;

  err = command(AES_DRIVER, TOCK_AES_CMD_CBC_ENC, 0, 0);
  if (err < TOCK_SUCCESS) return err;

  yield_for(&result.fired);

  // The IV for the next block is the ciphertext of this block.
  // Copy only on success so we don't clobber if it needs to re-execute.
  if (result.error == TOCK_SUCCESS) {
    memcpy(iv, buf, 16);
  }

  return result.error;
}


// Decrypt a block using CBC mode. Assumes both buf and iv are
// 16 bytes long.
static int aes128_decrypt_cbc_block(unsigned char* buf, unsigned char* iv) {
  int err;
  aes_data_t result = { .fired = false, .error = TOCK_SUCCESS };
  char next_iv[16];

  // Next IV is this block's ciphertext, so save it
  memcpy(next_iv, buf, 16);

  err = tock_aes128_set_callback(aes_cb, &result);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_input(buf, 16);
  if (err < TOCK_SUCCESS) return err;

  err = tock_aes128_set_ctr(iv, 16);
  if (err < TOCK_SUCCESS) return err;

  err = command(AES_DRIVER, TOCK_AES_CMD_CBC_DEC, 0, 0);
  if (err < TOCK_SUCCESS) return err;

  yield_for(&result.fired);

  // Copy this block's ciphertext to IV
  // Copy only on success so we don't clobber if it needs to re-execute.
  if (result.error == TOCK_SUCCESS) {
    memcpy(iv, next_iv, 16);
  }

  return result.error;
}


int tock_aes128_encrypt_cbc_sync(unsigned char* buf, unsigned char buf_len,
                                 unsigned char* iv, unsigned char iv_len) {
  if ((buf_len % 16 != 0) || (iv_len != 16)) {
    return TOCK_ESIZE;
  }

  // Just decrypt each block
  int err = TOCK_SUCCESS;
  for (int i = 0; i < buf_len; i+= 16) {
    err = aes128_encrypt_cbc_block(buf + i, iv);
    if (err != TOCK_SUCCESS){
      return err;
    }
  }
  return err;
}


int tock_aes128_decrypt_cbc_sync(unsigned char* buf, unsigned char buf_len,
                                 unsigned char* iv, unsigned char iv_len) {
  if ((buf_len % 16 != 0) || (iv_len != 16)) {
    return TOCK_ESIZE;
  }

  // Just decrypt each block
  int err = TOCK_SUCCESS;
  for (int i = 0; i < buf_len; i+= 16) {
    err = aes128_decrypt_cbc_block(buf + i, iv);
    if (err != TOCK_SUCCESS){
      return err;
    }
  }
  return err;
}
