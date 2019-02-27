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

#ifndef __EC_INCLUDE_TRNG_H
#define __EC_INCLUDE_TRNG_H

#include <stddef.h>
#include <stdint.h>
#include "fips_err.h"

/**
 * Initialize the true random number generator in a FIPS-compliant
 * mode.
 **/
void fips_trng_startup(void);

/**
 * Retrieve a FIPS-compliant 32 bit random value.
 **/
uint32_t fips_rand(void);

/**
 * Output len random bytes into buffer.
 **/
void rand_bytes(void *buffer, size_t len);

#endif /* __EC_INCLUDE_TRNG_H */
