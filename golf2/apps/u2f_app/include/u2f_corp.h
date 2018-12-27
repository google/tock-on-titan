/**
 * Copyright (c) 2016 The Chromium OS Authors. All rights reserved.
 * Use of this source code is governed by a BSD-style license that can be
 * found in the LICENSE file.
 *
 * This header provides definitions for the U2F protocol layer for
 * Corp Gnubbies. Official FIDO-compliant definitions are located in
 * "u2f.h".
 *
 */

#ifndef __U2F_UTIL_H_INCLUDED__
#define __U2F_UTIL_H_INCLUDED__

#ifndef __NO_PRAGMA_PACK
#pragma pack(push, 1)
#endif

#include "u2f.h"
#include "u2f_hid.h"

/* Additions to interface w/ u2f_transport.c */
/* ASN1DER_INTS are signed, big-endian (LSB last), minimal
   variable-length encoded. */
typedef struct {
  uint8_t fmt; /* 0x02 */
  uint8_t len;
  // potential leading 0 => +1
  uint8_t bytes[U2F_EC_KEY_SIZE + 1];
} ASN1DER_INT;

typedef struct {
  uint8_t fmt; /* 0x30 */
  uint8_t len;
  ASN1DER_INT r;
  ASN1DER_INT s;
} ASN1DER_P256_SIG;

/* APDU fields to pass around */
typedef struct {
  uint8_t p1;
  uint8_t p2;
  uint16_t len;
  const uint8_t *data;
} APDU;

/* P1 flags */
#define G2F_ATTEST 0x80  /* fixed attestation cert */
#define G2F_TUP 0x01     /* user presence required */
#define G2F_CONSUME 0x02 /* consume presence */
#define G2F_CHECK 0x04   /* test keyhandle */

/* KH covert channel flags */
#define G2F_KH_VERSION 0x01 /* fw version encoding present */

/* Non-spec commands */
#define G2F_SYSTEM_INFO 0x11 /* System/applet info command */
#define G2F_SELECT 0xa4      /* Applet context select */

#define G2F_PIN_SIGN 0x40 /* SSH applet commands */
#define G2F_PIN_GEN 0x41
#define G2F_PIN_UNLOCK 0x42
#define G2F_PIN_PUBKEY 0x43
#define G2F_PIN_DECYPT 0x44
#define G2F_ECR_READ 0x50
#define G2F_ECR_WRITE 0x51
#define G2F_CRT_WRITE 0x60
#define G2F_CRT_READ 0x61
#define G2F_RSA_LOAD 0x66

/* Response buffer offset */
#define SW_OFFSET 2

#define SYSTEM_INFO_LEN 8

/* Non-spec command status responses */
#define U2F_SW_CLA_NOT_SUPPORTED 0x6E00
#define U2F_SW_WRONG_LENGTH 0x6700
#define U2F_SW_WTF 0x6f00
/* status codes from ISO7816 */
#define U2F_SW_INCORRECT_P1P2 0x6A86
#define U2F_SW_FILE_FULL 0x6A84
#define U2F_SW_PIN_TRIES_REMAINING 0x63c0
#define U2F_SW_SECURITY_STATUS_NOT_SATISFIED 0x6982
#define U2F_SW_RECORD_NOT_FOUND 0x6A83

/* Inter-task communication */
/* WARNING: task global namespace. DCRYPTO has (1) */
#define TASK_EVENT_FRAME TASK_EVENT_CUSTOM(2)

uint16_t apdu_rcv(const uint8_t *in, uint16_t in_len, uint8_t *out);

/* Encryption constants */
#define AES_BLOCK_LEN 16
#define KH_LEN 64

/* Attestation certificate constants */
#define SN_VERSION 0x02

#ifndef __NO_PRAGMA_PACK
#pragma pack(pop)
#endif

#endif /* __U2F_UTIL_H_INCLUDED__ */
