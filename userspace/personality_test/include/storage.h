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
void get_personality(perso_st* id);
/* persist to flash */
int set_personality(const perso_st* id);

/* verify chksum; function of data and keyladder */
int check_personality(const perso_st* id);
/* draw from trng and compute all fields afresh */
int new_personality(perso_st* id);

#endif
