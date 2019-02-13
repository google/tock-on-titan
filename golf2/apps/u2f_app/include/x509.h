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

#ifndef __CROS_EC_INCLUDE_X509_H
#define __CROS_EC_INCLUDE_X509_H

#include <stddef.h>

//#include "util.h"

#ifdef CONFIG_FIPS
#include "p256.h"
#include "sha256.h"
#else
#include "dcrypto.h"
#endif

/**
 * Top-level construction of the fob attestation
 * certificate. Certificate is ASN.1 DER encoded.
 *
 * @param d Secret key
 * @param pk_x Public key x-coord
 * @param pk_y Public key y-coord
 * @param unique != 0 to generate a device individual certificate
 * @param cert Pointer to the output byte buffer
 * @param n Size of the output byte buffer
 *
 * @return Number of bytes written to *cert
 */
int generate_cert(const p256_int* d, const p256_int* pk_x, const p256_int* pk_y,
                  int unique, uint8_t* cert, const int n);

int anonymous_cert(const p256_int* d, const p256_int* pk_x,
                   const p256_int* pk_y, uint8_t* cert, const int n);

int individual_cert(uint8_t* cert, const int n);

/**
 * Generate the individual ECDSA keypair.
 *
 * @param d p256_int pointer to ECDSA private key
 * @param pk_x p256_int pointer to public key point (can be NULL)
 * @param pk_y p256_int pointer to public key point (can be NULL)
 * @param salt (can be NULL)
 *
 * @return
 * EC_SUCCESS or EC_ERROR_UNKNOWN
 */
int individual_keypair(p256_int* d, p256_int* pk_x, p256_int* pk_y,
                       const uint32_t salt[8]);

#endif
