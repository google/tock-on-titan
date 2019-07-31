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

#ifndef _F_DRBG_H__
#define _F_DRBG_H__

#include <stddef.h>
#include <stdint.h>
#include "fips_err.h"

// 800-90A HMAC-SHA256 DRBG
// http://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-90Ar1.pdf
typedef struct {
  uint32_t K[8];
  uint32_t V[8];
  size_t reseed_counter;
} DRBG;

void DRBG_init(DRBG* ctx, const void* p0, size_t p0_len, const void* p1,
               size_t p1_len, const void* p2, size_t p2_len);

void DRBG_exit(DRBG* ctx);

void DRBG_reseed(DRBG* ctx, const void* p0, size_t p0_len, const void* p1,
                 size_t p1_len, const void* p2, size_t p2_len);

void DRBG_update(DRBG* ctx, const void* p0, size_t p0_len, const void* p1,
                 size_t p1_len, const void* p2, size_t p2_len);

int DRBG_generate(DRBG* ctx, void* output, size_t output_len,
                  const void* input, size_t input_len);

#endif  // _F_DRBG_H__
