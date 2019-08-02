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

#ifndef __FIPS_HEALTH_TESTS_H__
#define __FIPS_HEALTH_TESTS_H__

#include "stdint.h"

/* TRNG health tests */
/* (1) Stuck-bit */
#define TRNG_INIT_BITS 4096
#define TRNG_INIT_WORDS (TRNG_INIT_BITS / 32)

// c = ceil(1 + (-log alpha)/H); alpha = 2^-40, H = 1.0.
#define RCT_CUTOFF_BITS 41
#define RCT_CUTOFF_WORDS ((RCT_CUTOFF_BITS + 31) / 32)

int rct_is_initialized(void);
void repetition_count_test(uint32_t val);
int repetition_count_test_n(uint32_t *in, uint32_t n);

/* (2) Adaptive Proportion */
#define WINDOW_SIZE_BITS 1024  // binary trng
#define WINDOW_SIZE_NWORDS ((WINDOW_SIZE_BITS + 31) / 32)

// H = 1.0
#define APT_CUTOFF_BITS 624

int apt_is_initialized(void);
void adaptive_proportion_test(uint32_t val);

#endif  // __FIPS_HEALTH_TESTS_H__
