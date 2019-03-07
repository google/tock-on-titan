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

#pragma once

#include "tock.h"

#ifdef __cplusplus
extern "C" {
#endif

// Configures the encryption key to be used for encryption and decryption.
//
// key - a buffer containing the key (must be 16 bytes)
// len - length of the buffer (must be 16)
int tock_aes128_set_key(const unsigned char* key, unsigned char len);


// Encrypts a payload according to aes-128 counter-mode. The counter
// stored in ctr is incremented for each block encrypted in a single call.
// The caller is responsible for incrementing/managing the counter across
// calls. For example, if the function is called to encrypt 64 bytes
// (4 blocks) with a counter of 17, the four blocks will be encrypted with
// counter values 17, 18, 19, 20. When the call returns, ctr will store
// 21. The ciphertext is put into buf.
//
// buf      - buffer to encrypt (must be N*16 bytes)
// buf_len  - length of the buffer to encrypt
// ctr      - buffer with the initial counter (must be 16 bytes)
// ctr_len  - length of buffer with the initial counter (must be 16)
int tock_aes128_encrypt_ctr_sync(unsigned char* buf, unsigned char buf_len,
                                 unsigned char* ctr, unsigned char ctr_len);

// Decrypts a payload according to aes-128 counter-mode. The counter
// stored in ctr is incremented for each block decrypted in a single call.
// The caller is responsible for incrementing/managing the counter across
// calls. For example, if the function is called to decrypt 64 bytes
// (4 blocks) with a counter of 17, the four blocks will be decrypted with
// counter values 17, 18, 19, 20. When the call returns, ctr will store
// 21. The cleartext is put into buf.
//
// buf      - buffer to decrypt (must be N*16 bytes, will be zero-padded)
// buf_len  - length of the buffer to decrypt
// ctr      - buffer with the initial counter (must be 16 bytes)
// ctr_len  - length of buffer with the initial counter (must be 16)
int tock_aes128_decrypt_ctr_sync(unsigned char* buf, unsigned char buf_len,
                                 unsigned char* ctr, unsigned char ctr_len);

// Encrypts a payload according to aes-128 electronic codebook mode. Note
// that this encryption mode is generally frowned upon, two identical
// cleartexts have the same ciphertext (it leaks information). The
// ciphertext is put into buf.
//
// buf      - buffer to encrypt (must be N*16 bytes)
// buf_len  - length of the buffer to encrypt
// ctr      - buffer with the initial counter (must be 16 bytes)
// ctr_len  - length of buffer with the initial counter (must be 16 bytes)
int tock_aes128_encrypt_ecb_sync(unsigned char* buf, unsigned char buf_len);

// Decrypts a payload according to aes-128 electronic codebook mode. Note
// that this encryption mode is generally frowned upon, two identical
// cleartexts have the same ciphertext (it leaks information). The cleartext
// is put into buf.
//
// buf      - buffer to decrypt (must be N*16 bytes)
// buf_len  - length of the buffer to decrypt
// ctr      - buffer with the initial counter (must be 16 bytes)
// ctr_len  - length of buffer with the initial counter (must be 16)
int tock_aes128_decrypt_ecb_sync(unsigned char* buf, unsigned char buf_len);


// Encrypts a payload according to aes-128 cipher-block-chaining mode. The
// caller passes the generated ciphertext of block N as the initialization
// vector for block N+1. The ciphertext is put into buf. When the call
// returns, the ciphertext of the last block encrypted is stored in iv.
//
// buf      - buffer to encrypt (must be N*16 bytes)
// buf_len  - length of the buffer to encrypt
// iv       - the initialization vector (must be 16 bytes)
// iv_len   - length of buffer with the IV (must be 16 bytes)
int tock_aes128_encrypt_cbc_sync(unsigned char* buf, unsigned char buf_len,
                                 unsigned char* iv, unsigned char iv_len);

// Decrypts a payload according to aes-128 cipher-block-chaining mode. The
// caller passes the ciphertext of block N as the initialization vector
// of block N+1. The cleartext is put into buf. When the call returns,
// the ciphertext of the last block decrypted is stored in iv.
//
// buf      - buffer to decrypt (must be N*16 bytes)
// buf_len  - length of the buffer to decrypt
// iv       - the initialization vector (must be 16 bytes)
// iv_len   - length of buffer with the IV (must be 16 bytes)
int tock_aes128_decrypt_cbc_sync(unsigned char* buf, unsigned char buf_len,
                                 unsigned char* iv, unsigned char iv_len);


#ifdef __cplusplus
}
#endif
