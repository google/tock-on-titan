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

#define AES_DRIVER 0x40000

#define TOCK_AES_CMD_CHECK 0
#define TOCK_AES_CMD_ECB_ENC 1
#define TOCK_AES_CMD_ECB_DEC 2
#define TOCK_AES_CMD_CTR_ENC 3
#define TOCK_AES_CMD_CTR_DEC 4
#define TOCK_AES_CMD_CBC_ENC 5
#define TOCK_AES_CMD_CBC_DEC 6

#define TOCK_AES_ALLOW_KEY    0
#define TOCK_AES_ALLOW_INPUT  1
#define TOCK_AES_ALLOW_OUTPUT 2
#define TOCK_AES_ALLOW_IVCTR  3

#define TOCK_AES_SUBSCRIBE_CRYPT 0

// function called by the encryption or decryption operation when they are
// finished
//
// callback       - pointer to function to be called
// callback_args  - pointer to data provided to the callback
int aes128_set_callback(subscribe_cb callback, void *callback_args);


// configures a buffer with data to be used for encryption or decryption
//
// data           - buffer with data
// len            - length of the data buffer
int aes128_set_data(const unsigned char *data, unsigned char len);


// configures a buffer with the initial counter to be used for encryption or
// decryption
//
// ctr            - buffer with initial counter
// len            - length of the initial counter buffer
int aes128_set_ctr(const unsigned char *ctr, unsigned char len);


// Internal function to trigger encryption operation.
// Note that this has no effect if not aes128_set_data() and aes128_set_ctr()
// have been invoked
int aes128_encrypt_start(void);


// Internal function to trigger decryption operation.
// Note that this has no effect if not aes128_set_data() and aes128_set_ctr()
// have been invoked
int aes128_decrypt_start(void);


// decrypts a payload according to aes-128 counter-mode asynchronously
//
// buf      - buffer to decrypt (currently max 128 bytes are supported)
// buf_len  - length of the buffer to decrypt
// ctr      - buffer with the initial counter (should be 16 bytes)
// ctr_len  - length of buffer with the initial counter (should be 16 bytes)
// callback - callback handler to be invoked when the operation is finished
int aes128_encrypt_ctr(const unsigned char* buf, unsigned char buf_len,
    const unsigned char* ctr, unsigned char ctr_len, subscribe_cb callback);


// decrypts a payload according to aes-128 counter-mode asynchronously
//
// buf      - buffer to decrypt (currently max 128 bytes are supported)
// buf_len  - length of the buffer to decrypt
// ctr      - buffer with the initial counter (should be 16 bytes)
// ctr_len  - length of buffer with the initial counter (should be 16 bytes)
// callback - callback handler to be invoked when the operation is finished
int aes128_decrypt_ctr(const unsigned char* buf, unsigned char buf_len,
    const unsigned char* ctr, unsigned char ctr_len, subscribe_cb callback);



// configures an encryption key to be used for encryption and decryption
//
// key - a buffer containing the key (should be 16 bytes for aes128)
// len - length of the buffer (should be 16 bytes for aes128)
int aes128_set_key_sync(const unsigned char* key, unsigned char len);


// encrypts a payload according to aes-128 counter-mode
//
// buf      - buffer to encrypt (currently max 128 bytes are supported)
// buf_len  - length of the buffer to encrypt
// ctr      - buffer with the initial counter (should be 16 bytes)
// ctr_len  - length of buffer with the initial counter (should be 16 bytes)
int aes128_encrypt_ctr_sync(const unsigned char* buf, unsigned char buf_len,
    const unsigned char* ctr, unsigned char ctr_len);


// decrypts a payload according to aes-128 counter-mode
//
// buf      - buffer to decrypt (currently max 128 bytes are supported)
// buf_len  - length of the buffer to decrypt
// ctr      - buffer with the initial counter (should be 16 bytes)
// ctr_len  - length of buffer with the initial counter (should be 16 bytes)
int aes128_decrypt_ctr_sync(const unsigned char* buf, unsigned char buf_len,
    const unsigned char* ctr, unsigned char ctr_len);

int aes128_encrypt_ecb_sync(unsigned char* buf, unsigned char buf_len);
int aes128_decrypt_ecb_sync(unsigned char* buf, unsigned char buf_len);

#ifdef __cplusplus
}
#endif
