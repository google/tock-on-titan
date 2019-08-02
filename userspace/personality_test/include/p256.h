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

#ifndef _F_P256_H__
#define _F_P256_H__

#include <stdint.h>
#include "drbg.h"

#define P256_BITSPERDIGIT 32
#define P256_NDIGITS 8
#define P256_NBYTES 32
#define P256_DIGIT(x, y) ((x)->a[y])

typedef uint32_t p256_digit;
typedef uint64_t p256_ddigit;
typedef int64_t p256_sddigit;

typedef struct {
  p256_digit a[P256_NDIGITS];
} p256_int;

extern const p256_int FIPS_SECP256r1_n;      // Curve order
extern const p256_int FIPS_SECP256r1_nMin2;  // Curve order - 2

// Return -1, 0, 1 for a < b, a == b or a > b respectively.
int fips_p256_cmp(const p256_int* a, const p256_int* b);
int fips_p256_is_zero(const p256_int* a);

// b = a + d. Returns carry, 0 or 1.
int fips_p256_add_d(const p256_int* a, p256_digit d, p256_int* b);

// 1 < k < |p256|
int fips_p256_pick(DRBG* drbg, p256_int* output, const void* data,
                   size_t data_len);

// overwrite w/ random bytes then zeroize
void fips_p256_clear(p256_int* output);

// Bytes to/from p256 points
void fips_p256_from_bin(const uint8_t src[], p256_int* dst);
void fips_p256_to_bin(const p256_int* src, uint8_t dst[]);

void fips_p256_init(p256_int* a);

#endif  //_F_P256_H__
