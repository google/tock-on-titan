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

#ifndef TOCK_DIGEST_H
#define TOCK_DIGEST_H

#include <stdlib.h>


typedef enum TockDigestMode {
  DIGEST_MODE_SHA1 = 0,
  DIGEST_MODE_SHA256 = 1,
  DIGEST_MODE_SHA256_HMAC = 2,
} TockDigestMode;

// TODO: Should be const, but currently not allowed by kernel
int tock_digest_set_input(void* buf, size_t len);
int tock_digest_set_output(void* buf, size_t len);

int tock_digest_check(void);

int tock_digest_hash_initialize(TockDigestMode mode);
int tock_digest_cert_initialize(uint32_t cert);

int tock_digest_hash_update(size_t n);
int tock_digest_hash_finalize(void);

// Return if the hash engine is busy
int tock_digest_busy(void);

int tock_digest_hash_easy(void* input_buf, size_t input_len,
                          void* output_buf, size_t output_len,
                          TockDigestMode mode);

// Rather than a normal digest, compute one using one of the keyladder
// "certificates", i.e. hidden secrets. These digests are always
// SHA256. Input and output are often NULL since this operation can
// generate hidden keys from hidden data.
//
// In the Cr52 code, this operation is kl_step; it's refactored to a
// digest driver call because it uses the SHA engine, so should go
// through the arbitration/busy checks that the SHA driver performs
// to prevent concurrent operations on the engine.
int tock_digest_with_cert(uint32_t cert,
                          void* input_buf, size_t input_len,
                          void* output_buf, size_t output_len);

#endif // TOCK_DIGEST_H
