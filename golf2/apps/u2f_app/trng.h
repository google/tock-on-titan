/* Copyright 2015 The Chromium OS Authors. All rights reserved.
 * Use of this source code is governed by a BSD-style license that can be
 * found in the LICENSE file.
 */
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
