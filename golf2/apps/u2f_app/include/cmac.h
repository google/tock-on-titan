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

#ifndef __CROS_EC_CMAC_H
#define __CROS_EC_CMAC_H

#include <stdint.h>
#include <stddef.h>

// cmac-aes-128
// output mac is aes block size (16 bytes)
int fips_cmac_verify(const void* key, const void* data, size_t data_len,
                     const void* mac, size_t mac_len);
int fips_cmac_generate(const void* key, const void* data, size_t data_len,
                       void* mac);

// write key to deep sleep storage
void cmac_save_key(const uint32_t cmac_key[4]);

// retrieve key from deep sleep storage
void cmac_restore_key(uint32_t cmac_key[4]);

#endif  // __CROS_EC_CMAC_H
