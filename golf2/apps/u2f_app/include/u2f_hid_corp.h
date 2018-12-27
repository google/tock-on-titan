/**
 * Copyright 2016 The Chromium OS Authors. All rights reserved.
 * Use of this source code is governed by a BSD-style license that can be
 * found in the LICENSE file.
 *
 * This header provides definitions for the U2F HID transport layer for
 * Corp Gnubbies. Official FIDO-compliant definitions are located in
 * "u2fhid.h".
 *
 */

/* U2F HID interface */
#ifndef __CROS_EC_U2F_HID_CORP_H
#define __CROS_EC_U2F_HID_CORP_H

/* Include the "official" FIDO header */
#include "u2f_hid.h"

#define USB_HID_SUBCLASS_NONE 0x00
#define USB_HID_PROTOCOL_NONE 0x00

void u2f_task(void);
void u2f_wakeup(void);
void usbu2f_get_frame(U2FHID_FRAME *frame_p);
int usbu2f_put_frame(const U2FHID_FRAME *frame_p);

typedef struct {
  uint32_t cid; /* current cid */
  uint8_t *data;
  uint8_t cmd;   /* current cmd */
  uint8_t seqno; /* expected seqno */
  uint16_t bcnt; /* expected total byte count */
} PENDING_MSG;

#define MSG_TIMEOUT 500000 /* us */

/*
 * max u2f msg payload
 * >= 2048 + 4, for DFU
 * >= 2315, for U2F_REGISTER_RESP
 */
#define MAX_BCNT (57 + 39 * 59)

/* Extended U2F HID commands */
#define U2FHID_SYSINFO (TYPE_INIT | 0x05)
#define U2FHID_DFU (TYPE_INIT | 0xba)

/* U2F HID extensions for USB update */
#define DFU_LOAD_BEGIN 0xfe
#define DFU_LOAD_EXTENDED 0xfd
#define DFU_LOAD_COMMIT 0xff

#define DFU_BLOCK_SIZE 0x800 /* 2048 B max */

/* Firmware query command */
#define ID_STRING_LEN 8
#define MAX_AID_LEN 28

/*
 * Last char of id will be 'A' or 'B', depending on where
 * current image is running.
 * This allows updater to pick the other image for update.
 * primary and secondary fields will be equal to SignedHeader fields.
 */
typedef struct {             /* 31 bytes */
  uint8_t id[ID_STRING_LEN]; /* "proto2__", "hg_evt_2", hg_dvt__", ... */
  uint8_t primaryMajor;      /* Fw epoch */
  uint8_t primaryMinor;      /* Fw major */
  uint8_t primaryBuild;      /* Fw minor */
  uint8_t secondaryMajor;    /* Applet epoch */
  uint8_t secondaryMinor;    /* Applet major */
  uint8_t secondaryBuild;    /* Applet minor */
  uint8_t aidLen;            /* Length of selected application identifier */
  /* [ epoch_ | major_ | minor_ | timestamp_  | fips_fatal | fips_fatal_lineno | sleep_cnt ]
   */
  uint8_t aid[MAX_AID_LEN]; /* Application identifier */
} SYSINFO;

#define STR(s) #s
#define XSTR(s) STR(s)

/* Default to DIR from board/DIR for the BOARD_STRING. Otherwise
 * defined in board.h. */
#ifndef BOARD_STRING
#define BOARD_STRING XSTR(BOARD)
#endif

#endif  // __CROS_EC_U2F_HID_CORP_H
