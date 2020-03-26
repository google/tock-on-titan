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

#include "fips_aes.h"
#include "sha256.h"
#include "storage.h"
#include "trng.h"
#include "u2f_hid_corp.h"

#include "digest_syscalls.h"
#include "h1_aes_syscalls.h"
#include "personality_syscalls.h"
#include "u2f_syscalls.h"
#include "nvcounter_syscalls.h"

#include "tock.h"
#include "rng.h"
#include "gpio.h"

#include "kl.h"

static uint32_t current_key[SHA256_DIGEST_WORDS];
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
static uint8_t block_len = AES128_BLOCK_CIPHER_KEY_SIZE;

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
  if (key_len == AES256_BLOCK_CIPHER_KEY_SIZE ||
      key_len == AES128_BLOCK_CIPHER_KEY_SIZE) {
    tock_aes_set_key(key, key_len);
    block_len = key_len;
  } else {
    printf("FAIL: aes_init passed a non-standard key length: %lu\n", key_len);
    return 0;
  }
  return 1;
}

#pragma GCC diagnostic ignored "-Wstack-usage="
int fips_aes_block(const uint8_t *in, uint8_t *out) {
  if (block_len != AES128_BLOCK_CIPHER_KEY_SIZE &&
      block_len != AES256_BLOCK_CIPHER_KEY_SIZE) {
    printf("fips_aes_block: invalid block length: %i\n", block_len);
    return 0;
  }
  if (cipher_mode == AES_CIPHER_MODE_CTR) {
    uint8_t iv[block_len];
    memcpy(iv, initialization_vector, block_len);
    memcpy(out, in, block_len);
    if (encrypt_mode == AES_ENCRYPT_MODE) {
      tock_aes_encrypt_ctr_sync(out, block_len, iv, block_len);
      increment_counter();
    } else {
      memcpy(out, in, block_len);
      tock_aes_decrypt_ctr_sync(out, block_len, iv, block_len);
      increment_counter();
    }
  } else if (cipher_mode == AES_CIPHER_MODE_CBC) {
    uint8_t iv[block_len];
    memcpy(iv, initialization_vector, block_len);
    memcpy(out, in, block_len);
    if (encrypt_mode == AES_ENCRYPT_MODE) {
      tock_aes_encrypt_cbc_sync(out, block_len, iv, block_len);
    } else {
      tock_aes_decrypt_cbc_sync(out, block_len, iv, block_len);
    }
  } else if (cipher_mode == AES_CIPHER_MODE_ECB) {
    memcpy(out, in, block_len);
    if (encrypt_mode == AES_ENCRYPT_MODE) {
      tock_aes_encrypt_ecb_sync(block_len, out, block_len);
    } else {
      tock_aes_decrypt_ecb_sync(block_len, out, block_len);
    }
  } else {
    printf("fips_aes_block: unsupported cipher mode: %i\n", cipher_mode);
    return 0;
  }
  return 1;
}

unsigned int increment_counter(void) {
  unsigned int counter;
  return tock_nvcounter_increment(&counter);
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
  printf("Button pressed (user contact)\n\n");
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
  if (consume) {
    tock_pop_clear();
  }
  return old;
}



/* Key ladder shims to Tock system calls; all of the calls boil down
   to kl_step, which invokes the KL system calls. */

// Value is SHA256(varname)
static uint32_t ISR2_SEED[8] = {0x704e9863, 0xf61c70d3, 0xd26f32e7,
                                0x294297e2, 0x4d1e939c, 0x64b3b6a8,
                                0xb5a31836, 0x1c1f1d7e};

static uint32_t KL_SEED_ATTEST[8] = {0x40640139, 0xcbfacf4a, 0xc2c2c27b,
                                     0x9f2d9cba, 0x8e3d41c3, 0x43bfe954,
                                     0x81cd534f, 0x23804b05};
static uint32_t KL_SEED_OBFS[8] = {0x4161c150, 0xb43c0c3c, 0xb1c62871,
                                   0xa2abfc84, 0x666d2091, 0x47c8f902,
                                   0xdc5b993e, 0xe89daab8};
static uint32_t KL_SEED_ORIGIN[8] = {0x06a7f502, 0x213c40c4, 0x5f3d4f19,
                                     0x52ca943b, 0x234e2fae, 0xddb6dc13,
                                     0xaa9556c0, 0xb2d538f1};
static uint32_t KL_SEED_SSH[8] = {0x2baf15a8, 0xaa452083, 0x08de59eb,
                                  0x44e5004c, 0x352acdaa, 0xc3ba7d54,
                                  0xc2d77c11, 0x79767216};

static int kl_step(uint32_t cert,
                   const uint32_t input[8],
                   uint32_t output[8]) {
  if (tock_digest_busy()) {
    printf("kl_step: DIGEST BUSY\n");
    return TOCK_EBUSY;
  } else {
    int rval = tock_digest_with_cert(cert,
                                     (void*)input, 32,
                                     (void*)output, 32);
    return rval;
  }
}


int kl_init(void) {
  uint32_t salt[8];
  int error = 0;
  size_t i;
  printf("Initializing keyladder.\n");
  // salt rsr some
  rand_bytes(salt, sizeof(salt));
  //error = error || kl_step(40, salt, NULL);
  rand_bytes(salt, sizeof(salt));
  error = error || kl_step(28, salt, NULL);

  // compute hcc2
  error = error || kl_step(0, NULL, NULL);
  error = error || kl_step(3, NULL, NULL);
  error = error || kl_step(4, NULL, NULL);
  error = error || kl_step(5, NULL, NULL);
  error = error || kl_step(7, NULL, NULL);
  error = error || kl_step(15, NULL, NULL);
  error = error || kl_step(20, NULL, NULL);
  for (i = 0; i < 254 + 1; ++i) error = error || kl_step(25, NULL, NULL);
  error = error || kl_step(34, ISR2_SEED, NULL);
  return error;

}

int kl_random(void* output) {
  int error = 0;
  uint32_t tmp[8];

  rand_bytes(tmp, 32);
  // TODO: 28 has limit of 512 invocations.. spread out?
  // error = error || kl_step(28, tmp, NULL);  // stir
  error = error || kl_step(27, tmp, tmp);  // extract

  if (!error) memcpy(output, tmp, 32);

  return error;
}

int kl_derive(const uint32_t salt[8] ,
              const uint32_t input[8] ,
              uint32_t output[8]) {
  int error = 0;

  error = error || kl_step(35, salt, NULL);     // isr2 -> usr0
  error = error || kl_step(38, input, output);  // hmac
  return error;
}

int kl_derive_attest(const uint32_t input[8],
                     uint32_t output[8]) {
  return kl_derive(KL_SEED_ATTEST, input, output);
}

int kl_derive_obfs(const uint32_t input[8],
                   uint32_t output[8]) {
  return kl_derive(KL_SEED_OBFS, input, output);
}

int kl_derive_origin(const uint32_t input[8],
                     uint32_t output[8]) {
  return kl_derive(KL_SEED_ORIGIN, input, output);
}

int kl_derive_ssh(const uint32_t input[8] ,
                  uint32_t output[8]) {
  return kl_derive(KL_SEED_SSH, input, output);
}

static perso_st personality;

perso_st* get_personality(void) {
  tock_get_personality(&personality);
  return &personality;
}

int set_personality(const perso_st* id) {
  int rval = tock_set_personality(id);
  if (rval == TOCK_SUCCESS) {
    return EC_SUCCESS;
  } else {
    return EC_ERROR_UNKNOWN;
  }
}
