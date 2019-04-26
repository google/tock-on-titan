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

/* U2F implementation-specific callbacks and parameters. */

#ifndef __CROS_EC_U2F_IMPL_H
#define __CROS_EC_U2F_IMPL_H

#include "common.h"
#include "crypto/p256.h"

// Call to initialize keyladder and other U2F state.
void u2f_init(void);

/* APDU fields to pass around */
struct apdu {
        uint8_t p1;
        uint8_t p2;
        uint16_t len;
        const uint8_t *data;
};

/*
 * Parses an APDU-framed message according to the u2f protocol.
 *
 * @return 0 on failure, output buffer's byte count on success.
 */
unsigned u2f_apdu_rcv(uint8_t *buffer, unsigned in_len, unsigned max_len);

/* ---- Physical presence ---- */

enum touch_state {
        POP_TOUCH_NO   = 0,  /* waiting for a user touch */
        POP_TOUCH_YES  = 1,  /* touch recorded and latched */
};

/*
 * Check whether the user presence event was latched.
 *
 * @param consume reset the latched touch event and the presence LED.
 * @return POP_TOUCH_NO or POP_TOUCH_YES.
 */
enum touch_state pop_check_presence(int consume);

/* ---- platform cryptography hooks ---- */

/**
 * Generate an origin-specific ECDSA keypair.
 *
 * Calculates a diversified chip-unique 256b value.
 *
 * @param seed ptr to store 32-byte seed to regenerate this key on this chip
 * @param d pointer to ECDSA private key
 * @param pk_x pointer to public key point
 * @param pk_y pointer to public key point
 *
 * @return EC_SUCCESS if a valid keypair was created.
 */
int u2f_origin_keypair(uint8_t *seed, p256_int *d,
                       p256_int *pk_x, p256_int *pk_y);

/**
 * Reconstitute the origin ECDSA private key from its seed.
 *
 * @param seed value returned by origin_keypair.
 * @param d ptr to store the retrieved private key.
 * @return EC_SUCCESS if we retrieved the key.
 */
int u2f_origin_key(const uint8_t *seed, p256_int *d);

/***
 * Generate a hardware derived 256b private key.
 *
 * @param kek ptr to store the generated key.
 * @param key_len size of the storage buffer. Should be 32 bytes.
 * @return EC_SUCCESS if a valid key was created.
 */
int u2f_gen_kek(const uint8_t *origin, uint8_t *kek, size_t key_len);

/**
 * Generate a hardware derived ECDSA keypair for individual attestation.
 *
 * @param seed ptr to store 32-byte seed to regenerate this key on this chip
 * @param d pointer to ECDSA private key
 * @param pk_x pointer to public key point
 * @param pk_y pointer to public key point
 *
 * @return EC_SUCCESS if a valid keypair was created.
 */
int g2f_individual_keypair(p256_int *d, p256_int *pk_x, p256_int *pk_y);

/* ---- protocol extensions ---- */

/* Use non-standard extensions to the U2F protocol */
int use_g2f(void);

/* Non-standardized command status responses */
#define U2F_SW_CLA_NOT_SUPPORTED 0x6E00
#define U2F_SW_WRONG_LENGTH 0x6700
#define U2F_SW_WTF 0x6f00
/* Additional flags for the P1 fields */
#define G2F_ATTEST 0x80  /* fixed attestation cert */
#define G2F_CONSUME 0x02 /* consume presence */

/* Vendor command to enable/disable the extensions */
#define U2F_VENDOR_MODE U2F_VENDOR_LAST

/* call extensions for unsupported U2F INS */
unsigned u2f_custom_dispatch(uint8_t ins, struct apdu apdu, uint8_t *buf,
                             unsigned *ret_len) __attribute__((weak));

#endif /* __CROS_EC_U2F_IMPL_H */
