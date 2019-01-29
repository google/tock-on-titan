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

#include "include/aes.h"
#include "include/sha256.h"
#include "include/trng.h"
#include "include/u2f_hid_corp.h"

#include "include/digest_syscalls.h"
#include "include/u2f_syscalls.h"
#include "include/aes_ecb_syscalls.h"

#include "tock.h"
#include "rng.h"
//#include "aes.h"
#include "gpio.h"

static uint32_t current_key[SHA256_DIGEST_WORDS];
static uint32_t current_hmac[SHA256_DIGEST_WORDS];
static uint32_t current_digest[SHA256_DIGEST_WORDS];

void fips_hwHMAC256_init(const uint32_t key[SHA256_DIGEST_WORDS]) {
  for (unsigned int i = 0 ; i < SHA256_DIGEST_WORDS; i++) {
    current_key[i] = key[i];
  }
  tock_digest_set_input((void*)current_key, SHA256_DIGEST_SIZE);
  tock_digest_hash_initialize(DIGEST_MODE_SHA256_HMAC);
}

void fips_hwSHA256_update(const void* data, size_t n) {
  tock_digest_set_input((void*)data, n);
  tock_digest_hash_update(n);
}

void fips_hwSHA256_init(void) {
  tock_digest_hash_initialize(DIGEST_MODE_SHA256);
  tock_digest_set_output(current_digest, SHA256_DIGEST_SIZE);
}

const uint8_t* fips_hwSHA256_final(uint32_t* output) {
  tock_digest_set_output(output, SHA256_DIGEST_SIZE);
  tock_digest_hash_finalize();
  return (uint8_t*)output;
}

static enum AES_encrypt_mode encrypt_mode = AES_ENCRYPT_MODE;
static enum AES_cipher_mode cipher_mode = AES_CIPHER_MODE_CTR;
static const uint8_t* initialization_vector = NULL;

int fips_aes_init(const uint8_t *key, uint32_t key_len, const uint8_t *iv,
                  enum AES_cipher_mode c_mode, enum AES_encrypt_mode e_mode) {
  if (cipher_mode != AES_CIPHER_MODE_CTR &&
      cipher_mode != AES_CIPHER_MODE_CBC &&
      cipher_mode != AES_CIPHER_MODE_ECB) {
    printf("fips_aes_init: unsupported cipher mode: %i\n", c_mode);
    printf("  supports CTR (%i), CBC (%i) and ECB (%i)\n", AES_CIPHER_MODE_CTR, AES_CIPHER_MODE_CBC, AES_CIPHER_MODE_ECB);
    return 0;
  }
  encrypt_mode = e_mode;
  cipher_mode = c_mode;
  initialization_vector = iv;

  // fips_aes_init takes the key_len in bits, but Tock expects it in bytes;
  // convert here.
  key_len = key_len / 8;
  if (key_len == AES256_BLOCK_CIPHER_KEY_SIZE) {
    tock_aes128_set_key_sync(key, key_len);
  } else if (key_len == AES256_BLOCK_CIPHER_KEY_SIZE/2) {
    tock_aes128_set_key_sync(key, key_len);
  } else {
    printf("FAIL: aes_init passed a non-standard key length: %lu\n", key_len);
    return 0;
  }
  return 1;
}

int fips_aes_block(const uint8_t *in, uint8_t *out) {
  if (cipher_mode == AES_CIPHER_MODE_CTR) {
    if (encrypt_mode == AES_ENCRYPT_MODE) {
      memcpy(out, in, 16);
      tock_aes128_encrypt_ctr_sync(out, 16, initialization_vector, 16);
      increment_counter();
    } else {
      memcpy(out, in, 16);
      tock_aes128_decrypt_ctr_sync(out, 16, initialization_vector, 16);
      increment_counter();
    }
  } else if (cipher_mode == AES_CIPHER_MODE_CBC) {
    if (encrypt_mode == AES_ENCRYPT_MODE) {
      memcpy(out, in, 16);
      tock_aes128_encrypt_cbc_sync(out, 16, initialization_vector, 16);
    } else {
      memcpy(out, in, 16);
      tock_aes128_decrypt_cbc_sync(out, 16, initialization_vector, 16);
    }
  } else if (cipher_mode == AES_CIPHER_MODE_ECB) {
    if (encrypt_mode == AES_ENCRYPT_MODE) {
      memcpy(out, in, 16);
      tock_aes128_encrypt_ecb_sync(out, 16);
    } else {
      memcpy(out, in, 16);
      tock_aes128_decrypt_ecb_sync(out, 16);
    }
  } else {
    printf("fips_aes_block: unsupported cipher mode: %i\n", cipher_mode);
    return 0;
  }
  return 1;
}

static int counter = 0;

int increment_counter(void) {
  counter++;
  return counter;
}

int usbu2f_put_frame(const U2FHID_FRAME* frame_p) {
  //printf("calling tock_u2f_transmit\n");
  tock_u2f_transmit((void*)frame_p, sizeof(U2FHID_FRAME));
  //printf("returned from tock_u2f_transmit\n");
  return 0;
}

void usbu2f_get_frame(U2FHID_FRAME *frame_p) {
  tock_u2f_receive((void*)frame_p, sizeof(U2FHID_FRAME));
}

uint32_t tock_chip_dev_id0(void) {
  return 0xdeadbeef;
}

uint32_t tock_chip_dev_id1(void) {
  return 0x600613;
}

int tock_chip_category(void) {
  return 0x0702;
}




void pop_falling_callback(int __attribute__((unused)) arg1,
                          int __attribute__((unused)) arg2,
                          int __attribute__((unused)) arg3,
                          void* __attribute__((unused)) data);

void pop_falling_callback(int __attribute__((unused)) arg1,
                          int __attribute__((unused)) arg2,
                          int __attribute__((unused)) arg3,
                          void* __attribute__((unused)) data) {
  printf("Pop callback\n");
  tock_pop_set();
}


static enum touch_state touch_latch = POP_TOUCH_NO;

void tock_pop_enable_detection(void) {
  gpio_enable_input(1, PullUp);
  gpio_interrupt_callback(pop_falling_callback, NULL);
  gpio_enable_interrupt(1, FallingEdge);
}

void tock_pop_set(void) {
  touch_latch = POP_TOUCH_YES;
}

void tock_pop_clear(void) {
  touch_latch = POP_TOUCH_NO;
}

enum touch_state tock_pop_check_presence(int consume) {
  enum touch_state old = touch_latch;
  printf("pop_check_presence consume=%i, returning %i\n", consume, old);
  if (consume) {
    tock_pop_clear();
  }
  return old;
}
