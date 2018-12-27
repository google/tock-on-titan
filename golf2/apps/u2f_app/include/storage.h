/*
 * Copyright 2015 The Chromium OS Authors. All rights reserved.
 * Use of this source code is governed by a BSD-style license that can be
 * found in the LICENSE file.
 */

#ifndef __CROS_EC_INCLUDE_STORAGE_H
#define __CROS_EC_INCLUDE_STORAGE_H

#include <stddef.h>

#include "p256.h"

/* Return the start and size of the dedicated storage memory */
void storage_info(uint32_t* base, uint32_t* size);

/* robust incrementing counter */
uint32_t flash_ctr_incr(void);

/* individual attestation data */
typedef struct {
  uint32_t chksum[8];
  uint32_t salt[8];
  p256_int pub_x;
  p256_int pub_y;
  uint32_t cert_hash[8];
  size_t cert_len;
  uint8_t cert[2048 - 4 - 5 * 32];
} perso_st;

/* get ptr to data in flash */
const perso_st* get_personality(void);
/* persist to flash */
int set_personality(const perso_st* id);

/* verify chksum; function of data and keyladder */
int check_personality(const perso_st* id);
/* draw from trng and compute all fields afresh */
int new_personality(perso_st* id);

#if defined(CONFIG_G2F)

/* 10 try pin */
int ownerpin_init(const void* data, size_t data_len);
int ownerpin_check(const void* data, size_t data_len);
int ownerpin_change(const void* data, size_t data_len);
const uint32_t* ownerpin_salt(void);

uint16_t cert_write(uint8_t which, uint8_t blockno, const uint8_t* data,
                    uint16_t size);
uint16_t cert_read(uint8_t which, uint8_t blockno, uint8_t* out,
                   uint16_t* out_len);

/*
 * Emergency credential (ecreds) storage.
 * It can store up to ECRED_BLOCK_COUNT blocks of ECRED_BLOCK_SIZE bytes.
 */
#define ECRED_BLOCK_SIZE  1024
#define ECRED_BLOCK_COUNT   10

#define ECRED_BLOCK_WORDS (ECRED_BLOCK_SIZE / sizeof(uint32_t))
#define ECRED_PAGE_COUNT  (ECRED_BLOCK_COUNT * ECRED_BLOCK_WORDS / PAGE_WORDS)

uint16_t ecr_write_block(unsigned blockno, const uint8_t* data, uint16_t size);
uint16_t ecr_read_block(unsigned blockno, uint8_t* out, uint16_t *out_len);

#endif  // CONFIG_G2F

#endif
