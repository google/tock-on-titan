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

#ifndef __P256_ECDSA_H__
#define __P256_ECDSA_H__

#include "common.h"
#include "p256.h"
#include "drbg.h"

//
// ECC.
//
// {r,s} := {kG mod n, (message + r*key)/k mod n}
//
// Note: message is a p256_int.
// Convert from a binary string using p256_from_bin().
int fips_p256_ecdsa_sign(DRBG *ctx, const p256_int *key,
                         const p256_int *message, p256_int *r, p256_int *s)
    __attribute__((warn_unused_result));

// Returns 0 if {r,s} is not a signature on message for
// public key {key_x,key_y}.
//
// Note: message is a p256_int.
// Convert from a binary string using p256_from_bin().
int fips_p256_ecdsa_verify(const p256_int *key_x, const p256_int *key_y,
                           const p256_int *message, const p256_int *r,
                           const p256_int *s)
    __attribute__((warn_unused_result));

// EC ops
int fips_p256_base_point_mul(const p256_int *k, p256_int *x, p256_int *y)
    __attribute__((warn_unused_result));
int fips_p256_point_mul(const p256_int *k, const p256_int *in_x,
                        const p256_int *in_y, p256_int *x, p256_int *y)
    __attribute__((warn_unused_result));

// Return whether point {x,y} is on curve.
int fips_p256_is_valid_point(const p256_int *x, const p256_int *y)
    __attribute__((warn_unused_result));

// Return !0 if key_bytes makes a valid 0 < d < p256-groupsize
int fips_p256_key_from_bytes(p256_int *x, p256_int *y, p256_int *d,
                             const uint8_t key_bytes[P256_NBYTES])
    __attribute__((warn_unused_result));

#endif  // __P256_ECDSA_H__
