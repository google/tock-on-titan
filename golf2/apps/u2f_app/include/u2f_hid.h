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

/*
 * USB HID definitions, from the proposed u2f specification header, 'u2f.h'.
 */

#ifndef __U2FHID_H_INCLUDED__
#define __U2FHID_H_INCLUDED__

#ifdef __cplusplus
extern "C" {
#endif

/* N.B. Should be <= to USB_MAX_PACKET_SIZE */
#define U2F_REPORT_SIZE 64

/* U2F frame layout; command and continuation */
#define CID_BROADCAST 0xffffffff

#define TYPE_MASK 0x80
/* Init-type frame */
#define TYPE_INIT 0x80
/* Continuation-type frame */
#define TYPE_CONT 0x00

typedef struct {
  uint32_t cid; /* channel ID */
  union {
    uint8_t type; /* frame type - b7 defines type */
    struct {
      uint8_t cmd;                       /* command - b7 set */
      uint8_t bcnth;                     /* msg byte count, high byte */
      uint8_t bcntl;                     /* msg byte count, low byte */
      uint8_t data[U2F_REPORT_SIZE - 7]; /* payload - (57) */
    } init;
    struct {
      uint8_t seq;                       /* sequence num - b7 cleared */
      uint8_t data[U2F_REPORT_SIZE - 5]; /* payload */
    } cont;
  };
} U2FHID_FRAME;

#define FRAME_TYPE(f) ((f).type & TYPE_MASK)
#define FRAME_CMD(f) ((f).init.cmd & ~TYPE_MASK)
#define MSG_LEN(f) ((f).init.bcnth * 256 + (f).init.bcntl)
#define FRAME_SEQ(f) ((f).cont.seq & ~TYPE_MASK)

/* U2F constants */
#define U2FHID_IF_VERSION 2
/* Default msg timeout in ms */
#define U2FHID_TRANS_TIMEOUT 3000

/* U2F HID native commands */
#define U2FHID_PING (TYPE_INIT | 0x01)
#define U2FHID_MSG (TYPE_INIT | 0x03)
#define U2FHID_LOCK (TYPE_INIT | 0x04)
#define U2FHID_INIT (TYPE_INIT | 0x06)
#define U2FHID_PROMPT (TYPE_INIT | 0x07) /* slow, long blink */
#define U2FHID_WINK (TYPE_INIT | 0x08)   /* fast, short blink */
#define U2FHID_SYNC (TYPE_INIT | 0x3c)
#define U2FHID_ERROR (TYPE_INIT | 0x3F)

#define U2FHID_VENDOR_FIRST (TYPE_INIT | 0x40) /* First vendor command */
#define U2FHID_VENDOR_LAST (TYPE_INIT | 0x7f)  /* Last vendor command */

/* U2FHID_INIT cmd defines */
#define INIT_NONCE_SIZE 8 /* Size of channel initialization challenge */
#define CAPFLAG_WINK 0x01 /* Supports WINK command */
#define CAPFLAG_LOCK 0x02 /* Supports application lock */

typedef struct {                  /* it's the cmd input FROM host */
  uint8_t nonce[INIT_NONCE_SIZE]; /* Client application nonce */
} U2FHID_INIT_REQ;

typedef struct {
  uint8_t nonce[INIT_NONCE_SIZE]; /* Client application nonce */
  uint32_t cid;                   /* Channel identifier */
  uint8_t versionInterface;       /* Interface version */
  uint8_t versionMajor;           /* Major version number */
  uint8_t versionMinor;           /* Minor version number */
  uint8_t versionBuild;           /* Build version number */
  uint8_t capFlags;               /* Capabilities flags */
} U2FHID_INIT_RESP;

/* U2FHID_SYNC cmd defines */
typedef struct {
  uint8_t nonce; /* Client application nonce */
} U2FHID_SYNC_REQ;

typedef struct {
  uint8_t nonce; /* Client application nonce */
} U2FHID_SYNC_RESP;

/* TODO: Move anything not in the "official" headers to u2f_corp.h or
 * u2fhid_corp.h"
 */

/* Low-level error codes. Return as negatives. */
/* Using Marius' error codes as canonical. */
#define ERR_NONE 0x00          /* No error */
#define ERR_INVALID_CMD 0x01   /* Invalid command */
#define ERR_INVALID_PAR 0x02   /* Invalid parameter */
#define ERR_INVALID_LEN 0x03   /* Invalid message length */
#define ERR_INVALID_SEQ 0x04   /* Invalid message sequencing */
#define ERR_MSG_TIMEOUT 0x05   /* Message has timed out */
#define ERR_CHANNEL_BUSY 0x06  /* Channel busy */
#define ERR_LOCK_REQUIRED 0x0a /* Command requires channel lock */
#define ERR_SYNC_FAIL 0x0b     /* SYNC command failed */
#define ERR_INVALID_CID 0x0b   /* Invalid CID, likely 0 */
#define ERR_OTHER 0x7f         /* Other unspecified error */

#ifdef __cplusplus
}
#endif

#endif /* __U2FHID_H_INCLUDED__ */
