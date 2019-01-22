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

// rfc4493 cmac-aes-128
//
#include "include/cmac.h"
#include "include/common.h"
#include "include/console.h"
#include "include/registers.h"
#include "include/aes.h"

static void _ls1(const uint8_t* in, uint8_t* out) {
  uint16_t accu = 0;
  int i;

  for (i = 15; i >= 0; --i) {
    accu |= (((uint16_t)in[i]) << 1);
    out[i] = accu;
    accu >>= 8;
  }
  if (accu) out[15] ^= 0x87;
}

static void _xor(uint8_t* inout, const uint8_t* in) {
  int i;

  for (i = 0; i < 16; ++i) {
    inout[i] ^= in[i];
  }
}

int fips_cmac_generate(const void* key, const void* data, size_t data_len,
                       void* tag) {
  uint8_t accu[16];
  uint8_t K1[16];
  uint8_t K2[16];
  int i, j;

  // setup key
  if (!fips_aes_init(key, 128, NULL, AES_CIPHER_MODE_ECB, AES_ENCRYPT_MODE)) {
    printf("ERROR: FIPS CMAC failed to init AES\n");
    return EC_ERROR_UNKNOWN;
  }

  // compute K1, K2
  memset(accu, 0, sizeof(accu));
  if (!fips_aes_block(accu, accu)) {
    printf("ERROR: FIPS CMAC failed to compute keys with AES.\n");
    return EC_ERROR_UNKNOWN;
  }
  _ls1(accu, K1);
  _ls1(K1, K2);

  // process data
  memset(accu, 0, sizeof(accu));
  for (i = 0; i < data_len; i += 16) {
    j = 0;
    for (; j < 16 && i + j < data_len; ++j) {
      accu[j] ^= ((const uint8_t*)data)[i + j];
    }
    if (j != 16) {
      accu[j] ^= 0x80;
      _xor(accu, K2);
    } else {
      if (i + j == data_len) {
        _xor(accu, K1);
      }
    }
    if (!fips_aes_block(accu, accu)) {
      printf("ERROR: FIPS CMAC failed to generate CMAC.\n");
      return EC_ERROR_UNKNOWN;
    }
  }

  memcpy(tag, accu, 16);
  return EC_SUCCESS;
}

int fips_cmac_verify(const void* key, const void* data, size_t data_len,
                     const void* mac, size_t mac_len) {
  uint8_t tag[16];
  int i, or = 0;

  // compute expected mac
  if (fips_cmac_generate(key, data, data_len, tag) != EC_SUCCESS)
    return EC_ERROR_UNKNOWN;

  // fixed-timing comparision
  for (i = 0; i < mac_len; ++i) or |= (tag[i] ^ ((const uint8_t*)mac)[i]);

  return or == 0 ? EC_SUCCESS : EC_ERROR_UNKNOWN;
}

static uint32_t stored_key[4];

void cmac_save_key(const uint32_t cmac_key[4]) {
  /*
  GREG32(PMU, PWRDN_SCRATCH24) = cmac_key[0];
  GREG32(PMU, PWRDN_SCRATCH25) = cmac_key[1];
  GREG32(PMU, PWRDN_SCRATCH26) = cmac_key[2];
  GREG32(PMU, PWRDN_SCRATCH27) = cmac_key[3];
  */
  stored_key[0] = cmac_key[0];
  stored_key[1] = cmac_key[1];
  stored_key[2] = cmac_key[2];
  stored_key[3] = cmac_key[3];
}

void cmac_restore_key(uint32_t cmac_key[4]) {
  cmac_key[0] = stored_key[0];
  cmac_key[1] = stored_key[1];
  cmac_key[2] = stored_key[2];
  cmac_key[3] = stored_key[3];

  /*
  cmac_key[0] = GREG32(PMU, PWRDN_SCRATCH24);
  cmac_key[1] = GREG32(PMU, PWRDN_SCRATCH25);
  cmac_key[2] = GREG32(PMU, PWRDN_SCRATCH26);
  cmac_key[3] = GREG32(PMU, PWRDN_SCRATCH27);
  */
}
