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

#include "include/fips.h"
#include "include/fips_err.h"
#include "include/sha256.h"
#include "include/p256_ecdsa.h"
#include "include/aes.h"
#include "include/cmac.h"
#include "include/trng.h"
#include "include/fips_crypto_tests.h"
#include "include/fips_health_tests.h"
#include "include/storage.h"

// libtock-c
#include "led.h"

/* Note: changing RCT_POOL will affect existing u2f and ssh keys! */
#ifdef CONFIG_RCT_ON_FIXED_POOL
#define RCT_POOL 1
#else
#define RCT_POOL 0
#endif

#define ARRAY_SIZE(x) (sizeof(x) / sizeof((x)[0]))

void _throw_fips_err(enum fips_err err) {
  /* accumulate */
  fips_fatal |= err;

  if (fips_fatal & FIPS_ERROR_MASK) {
    /* indicate */
    led_on(0);
  }
  printf("[fips_fatal %08X]\n", fips_fatal);
}

/**
 * Fatal FIPS failure global error. If set, FIPS crypto is
 * disabled. By extension U2F_REGISTER and U2F_AUTHENTICATE are
 * inoperable.
 */
int fips_fatal = FIPS_UNINITIALIZED;
int fips_fatal_lineno = 0;

/********************************************************/
/* Functions to pull factory-derived entropy from INFO1 */
/********************************************************/

/**
 * Fill a buffer with factory-derived entropy. Errors if you ask for
 * more entropy than is available.
 *
 * n - Number of bytes of entropy to retrieve.
 * rct - Flag to trigger the NIST-required repetition count test. If
 * set, this function consume n bytes + RCT_CUTOFF_WORDSs of entropy to
 * account for initial filling.
 */
static int fill_pool(void *out, size_t n, int rct) {
  uint i, words_n;
  uint32_t buf[FACTORY_ENTROPY_SIZE / sizeof(uint32_t) + RCT_CUTOFF_WORDS];

  /* Round up from bytes to words */
  words_n = (n + sizeof(uint32_t) - 1) / sizeof(uint32_t);
  /* Per FIPS, can't use the first RCT_CUTOFF_WORDS of randomness that */
  /* are checked by the health test. */
  words_n += (rct ? RCT_CUTOFF_WORDS : 0);

  /* Can't take more factory entropy than exists */
  if (words_n > ARRAY_SIZE(buf)) return EC_ERROR_UNKNOWN;

  flash_info_read_enable(FACTORY_ENTROPY_OFFSET, FACTORY_ENTROPY_SIZE);

  for (i = 0; i < words_n; i++) {
    if (flash_physical_info_read_word(
            FACTORY_ENTROPY_OFFSET + i * sizeof(uint32_t), buf + i) !=
        EC_SUCCESS) {
      return EC_ERROR_INVAL; /* Flash read INFO1 failed. */
    }
  }

  flash_info_read_disable();

  /* Stuck-bit test */
  if (rct) {
    if (repetition_count_test_n(buf, words_n) != EC_SUCCESS) {
      return EC_ERROR_UNKNOWN;
    }
  }

  memcpy(out, buf + (rct ? RCT_CUTOFF_WORDS : 0), n);

  return EC_SUCCESS;
}

/****************/
/* DRBG flavors */
/****************/

/**
 * DRBG 1 -- setup, teardown; long-lived secrets (origin-specific kp,
 * seeds, etc.). Seeded from fixed factory-derived entropy.
 *
 * Changes to its setup will affect existing u2f and ssh keys!
 */
void make_drbg1(DRBG *ctx) {
  uint8_t factory_rand[ENTROPY_128SEC + NONCE_128SEC] = {0x0A};

  /* Fill the pool w/ factory-derived entropy, init the DRBG, then
   * secure clear the pool */
  if (fill_pool(factory_rand, sizeof(factory_rand), RCT_POOL) != EC_SUCCESS) {
    throw_fips_err(FIPS_FATAL_TRNG);
  }
  DRBG_init(ctx, factory_rand, ENTROPY_128SEC, factory_rand + ENTROPY_128SEC,
            NONCE_128SEC, NULL, 0);
  rand_bytes(factory_rand, sizeof(factory_rand));
  memset(factory_rand, 0x0, sizeof(factory_rand));
}

/**
 * DRBG 2 -- seeded from trng. For ephemeral entropy needs.
 *
 * Can be changed w/o consequences to existing keys.
 */
void make_drbg2(DRBG *ctx) {
  uint8_t rng_buf[ENTROPY_128SEC + NONCE_128SEC] = {0x0B};

  rand_bytes(rng_buf, ENTROPY_128SEC + NONCE_128SEC);
  DRBG_init(ctx, rng_buf, ENTROPY_128SEC, rng_buf + ENTROPY_128SEC,
            NONCE_128SEC, NULL, 0);
  rand_bytes(rng_buf, sizeof(rng_buf));
  memset(rng_buf, 0x0, sizeof(rng_buf));
}

// Returns 0 on success
int fips_keygen(DRBG *drbg, p256_int *d, p256_int *x, p256_int *y,
                const void *addl_data, int addl_len) {
  // Index into the entropy of the primed drbg
  int rv = fips_p256_pick(drbg, d, addl_data, addl_len);
  if (rv) return rv;

  if (x == NULL || y == NULL) {
    return 0;
  } else {
    // Bump counter for new keys.
    // Fail hard and forever if entropy is depleted.
    if (flash_ctr_incr() == -1u) {
      throw_fips_err(FIPS_FATAL_TOO_MANY_KEYS);
      return EC_ERROR_UNKNOWN;
    }

    // Compute public key
    rv = (fips_p256_base_point_mul(d, x, y) == 0);

    // FIPS consistency check for new keypairs
    if (fips_ecdsa_consistency_test(x, y, d)) {
      throw_fips_err(FIPS_FATAL_ECDSA);
      return EC_ERROR_UNKNOWN;
    }
  }

  return rv;
}

/******************/
/* Initialization */
/******************/
/**
 * Single point of initialization for all FIPS-compliant
 * cryptography. Responsible for KATs, TRNG testing, and signalling a
 * fatal error.
 */
int init_fips(void) {
  DRBG ctx;
  p256_int x, y;
  printf("FIPS initialization start.\n");
  // SHA
  if (fips_sha256_kat()) {
    throw_fips_err(FIPS_FATAL_HMAC_SHA256);
    return EC_ERROR_UNKNOWN;
  }

  printf("PASS: FIPS SHA256.\n");
  // HMAC
  if (fips_hmac_sha256_kat()) {
    throw_fips_err(FIPS_FATAL_HMAC_SHA256);
    return EC_ERROR_UNKNOWN;
  }
  printf("PASS: FIPS HMAC SHA256.\n");

  // DRBG
  if (fips_hmac_drbg_instantiate_kat(&ctx)) {
    throw_fips_err(FIPS_FATAL_HMAC_DRBG);
    return EC_ERROR_UNKNOWN;
  }
  printf("PASS: FIPS HMAC DRBG instantiate\n");

  if (fips_hmac_drbg_reseed_kat(&ctx)) {
    throw_fips_err(FIPS_FATAL_HMAC_DRBG);
    return EC_ERROR_UNKNOWN;
  }
  printf("PASS: FIPS HMAC DRBG reseed\n");

  if (fips_hmac_drbg_generate_kat(&ctx)) {
    throw_fips_err(FIPS_FATAL_HMAC_DRBG);
    return EC_ERROR_UNKNOWN;
  }
  printf("PASS: FIPS HMAC DRBG generate\n");

  // CMAC
  if (fips_cmac_aes128_kat()) {
    throw_fips_err(FIPS_FATAL_CMAC_AES128);
    return EC_ERROR_UNKNOWN;
  }

  printf("PASS: FIPS CMAC AES128\n");

  // AES
  if (fips_aes128_kat()) {
    throw_fips_err(FIPS_FATAL_AES128);
    return EC_ERROR_UNKNOWN;
  }

  printf("PASS: FIPS AES128\n");
  // ECDSA
  /* (1) FIPS ECDSA Signature known-answer test:
   * Fix k, check for previously known r & s.
   * (2) P256 ECDSA Verify KAT:
   * Derive the public key from a fixed private key. Verify the
   * signature from the above signing KAT.
   * (3) Verify the signature. OK to reuse the sig from the KAT. */
  if (fips_ecdsa_sign_kat()) {
    throw_fips_err(FIPS_FATAL_ECDSA);
    return EC_ERROR_UNKNOWN;
  }
  printf("PASS: FIPS ECDSA\n");

  if (!fips_p256_base_point_mul(&fixed_d, &x, &y)) {
    throw_fips_err(FIPS_FATAL_ECDSA);
    return EC_ERROR_UNKNOWN;
  }
  printf("PASS: FIPS P256 multiply\n");

  if (!fips_p256_ecdsa_verify(&x, &y, &test_msg, &fixed_r, &fixed_s)) {
    throw_fips_err(FIPS_FATAL_ECDSA);
    return EC_ERROR_UNKNOWN;
  }
  printf("PASS: FIPS ECDSA verify\n");

  /* Here and only here */
  fips_fatal = FIPS_INITIALIZED;
  printf("FIPS initialization complete.\n");
  return EC_SUCCESS;
}
