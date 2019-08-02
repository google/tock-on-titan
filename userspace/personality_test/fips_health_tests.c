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

#include "fips_health_tests.h"
#include "fips_err.h"
#include "common.h"

/**************************/
/* NIST TRNG health tests */
/**************************/

/**
 * Check running 0 or 1 streaks to be within limit.
 * RCT_CUTOFF_BITS has to be >= 32 to function.
 */
static uint8_t __rct_count;
void repetition_count_test(uint32_t rnd) {
  static int last_clz;
  static int last_clo;
  uint32_t clz, ctz, clo, cto;

  clz = __builtin_clz(rnd);
  ctz = __builtin_ctz(rnd);
  clo = __builtin_clz(~rnd);
  cto = __builtin_ctz(~rnd);

  if (ctz + last_clz >= RCT_CUTOFF_BITS) throw_fips_err(FIPS_FATAL_TRNG);
  if (cto + last_clo >= RCT_CUTOFF_BITS) throw_fips_err(FIPS_FATAL_TRNG);

  last_clz = clz + ((!!rnd - 1) & last_clz);
  last_clo = clo + ((!!~rnd - 1) & last_clo);

  if (__rct_count < RCT_CUTOFF_WORDS) ++__rct_count;
}

int rct_is_initialized(void) { return __rct_count >= RCT_CUTOFF_WORDS; }

/**
 * Word-wise stuck-bit test for fixed entropy pools.
 *
 * NOTE: If this function is being used for FIPS-compatibility, you
 * will need to consume more entropy than you require to fill up the
 * pipeline. Filling the pipeline requires RCT_CUTOFF_WORDS.
 *
 */
int repetition_count_test_n(uint32_t *in, uint32_t n) {
  uint32_t last_clz = 0;
  uint32_t last_clo = 0;
  uint32_t i;

  // Checking for < RCT_CUTOFF words doesn't make sense.
  if (n < RCT_CUTOFF_WORDS) return EC_ERROR_UNKNOWN;

  for (i = 0; i < n; i++) {
    uint32_t rnd = *in++;
    uint32_t clz, ctz, clo, cto;

    clz = __builtin_clz(rnd);
    ctz = __builtin_ctz(rnd);
    clo = __builtin_clz(~rnd);
    cto = __builtin_ctz(~rnd);

    if (ctz + last_clz >= RCT_CUTOFF_BITS) return EC_ERROR_UNKNOWN;
    if (cto + last_clo >= RCT_CUTOFF_BITS) return EC_ERROR_UNKNOWN;

    last_clz = clz + ((!!rnd - 1) & last_clz);
    last_clo = clo + ((!!~rnd - 1) & last_clo);
  }

  return EC_SUCCESS;
}

#ifdef CONFIG_ENABLE_APT
static int misbalanced(uint32_t count) {
  return count > APT_CUTOFF_BITS || count < WINDOW_SIZE_BITS - APT_CUTOFF_BITS;
}

static int popcount(uint32_t x) {
  x = x - ((x >> 1) & 0x55555555);
  x = (x & 0x33333333) + ((x >> 2) & 0x33333333);
  x = (x + (x >> 4)) & 0x0F0F0F0F;
  x = x + (x >> 8);
  x = x + (x >> 16);
  return x & 0x0000003F;
}
#endif

/**
 * Sliding window that counts the relative proporition of ones
 * and zeros in the last 32 words. Throw a FIPS error if the relative
 * proportion exceeds bounds.
 */
static uint8_t __apt_initialized;
void adaptive_proportion_test(uint32_t val) {
#ifdef CONFIG_ENABLE_APT
  static uint8_t pops[WINDOW_SIZE_NWORDS];
  static uint32_t oldest;
  static uint32_t count;

  // update rolling count
  count -= pops[oldest];
  pops[oldest] = popcount(val);
  count += pops[oldest];
  if (++oldest >= WINDOW_SIZE_NWORDS) {
    __apt_initialized = 1;  // saw full window
    oldest = 0;
  }

  // check when initialized
  if (__apt_initialized != 0 && misbalanced(count)) {
    throw_fips_err(FIPS_FATAL_TRNG);
  }
#else
  __apt_initialized = 1;
#endif
}

int apt_is_initialized(void) { return __apt_initialized != 0; }
