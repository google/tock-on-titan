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

#ifndef __AES_H__
#define __AES_H__

#include "common.h"

#define CTRL_CTR_BIG_ENDIAN (__BYTE_ORDER__ == __ORDER_BIG_ENDIAN__)
#define CTRL_ENABLE 1
#define CTRL_ENCRYPT 1
#define CTRL_NO_SOFT_RESET 0

/*
 * Use this structure to avoid alignment problems with input and output
 * pointers.
 */
struct AES_access_helper {
  uint32_t udata;
} __packed;

/*
 * AES configuration settings
 */
enum AES_cipher_mode {
  AES_CIPHER_MODE_ECB = 0,
  AES_CIPHER_MODE_CTR = 1,
  AES_CIPHER_MODE_CBC = 2,
  AES_CIPHER_MODE_GCM = 3
};

enum AES_encrypt_mode { AES_DECRYPT_MODE = 0, AES_ENCRYPT_MODE = 1 };

/*
 * AES implementation, based on a hardware AES block.
 */
#define AES256_BLOCK_CIPHER_KEY_SIZE 32

int fips_aes_init(const uint8_t *key, uint32_t key_len, const uint8_t *iv,
                  enum AES_cipher_mode c_mode, enum AES_encrypt_mode e_mode);
int fips_aes_block(const uint8_t *in, uint8_t *out);

#endif  // __AES_H__
