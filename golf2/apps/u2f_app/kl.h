/* Copyright (c) 2016 Google.
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
