/*
 * Copyright 2015 The Chromium OS Authors. All rights reserved.
 * Use of this source code is governed by a BSD-style license that can be
 * found in the LICENSE file.
 */
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
