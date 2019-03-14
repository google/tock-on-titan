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

/*
 *
 * Key ladder functions
 *
 */

#ifndef __CROS_EC_KL_H
#define __CROS_EC_KL_H

/**
 * Setup
 * Call at init or any time other code has touched KL
 */
int kl_init(void) __attribute__((warn_unused_result));

/**
 * Get 256 bit of entropy
 */
int kl_random(void* output) __attribute__((warn_unused_result));

/**
 * Pull keys out of various branches
 */
int kl_derive(const uint32_t salt[8], const uint32_t input[8],
              uint32_t output[8]) __attribute__((warn_unused_result));
int kl_derive_attest(const uint32_t input[8], uint32_t output[8])
    __attribute__((warn_unused_result));
int kl_derive_obfs(const uint32_t input[8], uint32_t output[8])
    __attribute__((warn_unused_result));
int kl_derive_origin(const uint32_t input[8], uint32_t output[8])
    __attribute__((warn_unused_result));
int kl_derive_ssh(const uint32_t input[8], uint32_t output[8])
    __attribute__((warn_unused_result));

#endif /* __CROS_EC_KL_H */
