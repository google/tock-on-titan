/* Copyright (c) 2012 The Chromium OS Authors. All rights reserved.
 * Use of this source code is governed by a BSD-style license that can be
 * found in the LICENSE file.
 */

/* SHA-256 functions */

#ifndef __CROS_EC_SHA256_H
#define __CROS_EC_SHA256_H

#include "common.h"

#define SHA256_DIGEST_SIZE 32
#define SHA256_BLOCK_SIZE 64
#define SHA256_DIGEST_WORDS (SHA256_DIGEST_SIZE/sizeof(uint32_t))

/* SHA256 context */
struct sha256_ctx {
        uint32_t h[8];
        uint32_t tot_len;
        uint32_t len;
        uint8_t block[2 * SHA256_BLOCK_SIZE];
        uint8_t buf[SHA256_DIGEST_SIZE];  /* Used to store the final digest. */
};

#define LITE_SHA256_CTX struct sha256_ctx

void SHA256_init(struct sha256_ctx *ctx);
void SHA256_update(struct sha256_ctx *ctx, const uint8_t *data, uint32_t len);
uint8_t *SHA256_final(struct sha256_ctx *ctx);

void hmac_SHA256(uint8_t *output, const uint8_t *key, const int key_len,
                 const uint8_t *message, const int message_len);

uint8_t* tock_compat_sha256(const void* data, size_t len, uint8_t* digest);

void fips_hwHMAC256_init(const uint32_t[]);
void fips_hwSHA256_init(void);
void fips_hwSHA256_update(const void*, size_t);
const uint8_t* fips_hwSHA256_final(uint32_t[SHA256_DIGEST_WORDS]);

#endif  /* __CROS_EC_SHA256_H */
