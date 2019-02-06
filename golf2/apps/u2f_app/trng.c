/* Copyright 2015 The Chromium OS Authors. All rights reserved.
 * Use of this source code is governed by a BSD-style license that can be
 * found in the LICENSE file.
 */

#include "include/trng.h"
#include "include/fips.h"
#include "include/fips_err.h"
#include "include/fips_health_tests.h"

// Tock
#include "rng.h"

static uint32_t raw_rand(void);

#if defined(SECTION_IS_RO)
void init_trng(void) {
  // dummy function, just to keep Makefile happy.
}

uint32_t rand(void) {
  // dummy function, just to keep Makefile happy.
  return 0;
}
#endif

/*
 * FIPS-compliant TRNG initialization.
 * 1-bit alphabet (2 symbols)
 * No post-processing.
 */
static void fips_init_trng(void) {
  /*
  GWRITE(TRNG, POST_PROCESSING_CTRL, 0);
  // 1-bit sample size
  GWRITE(TRNG, SLICE_MAX_UPPER_LIMIT, 0);
  GWRITE(TRNG, SLICE_MIN_LOWER_LIMIT, 0);

  GWRITE(TRNG, TIMEOUT_COUNTER, 0x7ff);
  GWRITE(TRNG, TIMEOUT_MAX_TRY_NUM, 4);
  GWRITE(TRNG, POWER_DOWN_B, 1);
  GWRITE(TRNG, GO_EVENT, 1);
  */
}

static uint32_t raw_rand(void) {
  /*
  while (GREAD(TRNG, EMPTY)) {
    if (GREAD_FIELD(TRNG, FSM_STATE, FSM_IDLE)) {
      // TRNG timed out, restart
      GWRITE(TRNG, STOP_WORK, 1);
      GWRITE(TRNG, GO_EVENT, 1);
    }
  }

  return GREAD(TRNG, READ_DATA);
  */
  uint32_t val = 0xdeadbeef;
  rng_sync((uint8_t*)&val, 4, 4);
  return val;
}

/**
 * FIPS-compliant TRNG startup.
 * Runs start-up tests over 4K samples.
 * N.B. This function can set the global 'fips_error' variable.
 */
void fips_trng_startup(void) {
  int i;

  fips_init_trng();

  /* Startup tests per NIST SP800-90B, Section 4 */
  /* 4096 1-bit samples */
  for (i = 0; i < (TRNG_INIT_WORDS); i++) {
    uint32_t r = raw_rand();
    /* warm-up test #1: Repetition Count Test (aka Stuck-bit) */
    repetition_count_test(r);
    /* warm-up test #2: Adaptive Proportion Test */
    adaptive_proportion_test(r);
  }
}

/* N.B. This function can set the global 'fips_error' variable. */
uint32_t fips_rand(void) {
  uint32_t r = raw_rand();

  /* Add sample to continuous health tests */
  repetition_count_test(r);
  adaptive_proportion_test(r);

  return r;
}

void rand_bytes(void *buffer, size_t len) {
  uint8_t *buf = (uint8_t *)buffer;

  /*
   * Retrieve random numbers in 4 byte quantities and pack as many bytes
   * as needed into 'buffer'. If len is not divisible by 4, the
   * remaining random bytes get dropped.
   */
  while (len != 0) {
    uint32_t random_value = fips_rand();
    size_t n = len;

    if (n > sizeof(random_value)) n = sizeof(random_value);

    memcpy(buf, &random_value, n);
    buf += n;
    len -= n;
  }
}
