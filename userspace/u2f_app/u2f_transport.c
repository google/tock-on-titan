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

/*
 * NOTHING YET!
 *
 * TODO: How many errors should clear the channel state, all of them?
 * If the response is an error message, does that always wipe out the
 * channel?. Possible to stick this into the error handler? Easy to check
 * once it's all working.
 *
 *  TODO: Some errors expect the channel to be reset, some don't. Try
 *  and make this consistent.
 */

#include "kl.h"
#include "hid_dfu.h"
#include "trng.h"
#include "u2f_corp.h"
#include "u2f_hid_corp.h"


#include "fips.h"
#include "p256_ecdsa.h"
#include "fips_err.h"
#if defined(CONFIG_FIPS_TEST)
#include "fips-commands.h"
#endif

#include "console.h"

/* Next CID to allocate. */
static uint32_t next_CID = 0x1;
/* Channel lock variable -- lock CID for n seconds */
static uint32_t lock_CID;
/* CID of the channel waiting for timeout */
static uint32_t timeout_CID;

/* Maintains state for multi-packet transactions */
static PENDING_MSG pending;

static uint8_t rx_buffer[MAX_BCNT];
static uint8_t tx_buffer[MAX_BCNT];

static SYSINFO U2F_sysinfo;

/* Return 0 if success, 1 if last frame. */
static int consume_frame(U2FHID_FRAME *f_p) {
  int nreceived = 57 + pending.seqno * 59;
  uint8_t *p = rx_buffer + nreceived;

  memcpy(p, f_p->cont.data, 59);
  nreceived += 59;
  pending.seqno += 1;

  return nreceived >= pending.bcnt;
}

/* Send the U2F HID protocol error code back over the USB channel */
static void u2fhid_err(uint32_t cid, uint8_t errno) {
  U2FHID_FRAME r = {0, .init = {0, 0, 0, {0}}};

  /* Construct U2F HID error cmd response frame */
  r.cid = cid;
  r.init.cmd = U2FHID_ERROR;
  r.init.bcnth = 0;
  r.init.bcntl = 1;
  r.init.data[0] = errno;

  /* Send the response */
  usbu2f_put_frame(&r);
}

static void clear_pending(void) {
  pending.cid = 0;
  pending.data = NULL;
  pending.cmd = 0;
  pending.seqno = 0;
  pending.bcnt = 0;
}

// Note: timeouts are not used; they are vestigial from original U2F code.
/*
  static void u2fhid_timeout(void) {
  if (timeout_CID) {
    printf("%s: cid %04lx (pending %04lx)", __func__, timeout_CID, pending.cid);
    u2fhid_err(timeout_CID, ERR_MSG_TIMEOUT);
    timeout_CID = 0;
    clear_pending();
  }
}*/
//DECLARE_DEFERRED(u2fhid_timeout);

static void cancel_timeout(void) {
  if (timeout_CID) {
    timeout_CID = 0;
  }
  //hook_call_deferred(&u2fhid_timeout_data, -1);
}

/* Start/restart timeout for a given channel */
static void start_timeout(uint32_t cid) {
  timeout_CID = cid;
  //  hook_call_deferred(&u2fhid_timeout_data, MSG_TIMEOUT);
}

/* Spec 4.1.1
 * This command sends an encapsulated U2F message to the device. The
 * semantics of the data message [are] defined in the U2F protocol
 * specification.
 */
static int u2fhid_cmd_msg(const uint8_t *in, uint16_t in_len, uint8_t *out,
                          uint16_t *n) {
  /* U2F HID transport stripped, pass to U2F protocol. */
  *n = apdu_rcv(in, in_len, out);

  return EC_SUCCESS;
}

/* Spec 4.1.3
 * Sends a transaction to the device, which immediately echoes the
 * same data back. This command is defined to be [a] uniform function
 * for debugging, latency and performance measurements.
 */
static int u2fhid_cmd_ping(const uint8_t *in, const uint16_t n, uint8_t *out) {
  memcpy(out, in, n); /* Echo */

  return EC_SUCCESS;
}
/*
static void cancel_lock_timeout(void) {
  printf("Lock %04lx expired\n", lock_CID);
  lock_CID = 0;
  }*/
//DECLARE_DEFERRED(cancel_lock_timeout);

/* Spec 4.2.2
 * The lock command places an exclusive lock for one channel to
 * communicate with the device. As long as the lock is active, any
 * other channel trying to send a message will fail. In order to
 * prevent a stalling or crashing application [from locking] the device
 * indefinitely, a lock time up to 10 seconds [must] be set. An
 * application requiring a longer lock has to send repeating lock
 * commands to maintain the lock.
 */
static int u2fhid_cmd_lock(const uint32_t cid, const uint8_t duration) {
  if (!duration) {
    printf("Lock %04lx canceled\n", cid);
    lock_CID = 0;
    //hook_call_deferred(&cancel_lock_timeout_data, -1);
  } else {
    printf("Lock %04lx set for %d\n", cid, duration);
    //hook_call_deferred(&cancel_lock_timeout_data, duration * SECOND);
    lock_CID = cid;
  }

  return EC_SUCCESS;
}

/* U2F HID command WINK */
static int u2fhid_cmd_wink(void) {
  /* TODO: Frob the LED */
  //printf("\nWINK WINK\n");
  return EC_SUCCESS;
}

/* U2F HID command PROMPT */
static int u2fhid_cmd_prompt(void) {
  /* TODO: Frob the LED */
  //printf("\nPROMPT PROMPT\n");
  return EC_SUCCESS;
}

/* System information command */
static int u2fhid_cmd_sysinfo(uint8_t *out, uint16_t *n) {
  //uint32_t sleep_cnt = GREG32(PMU, PWRDN_SCRATCH17);
  uint32_t sleep_cnt = 0;

  /* Sample and cache fips_fatal, big endian */
  U2F_sysinfo.aid[16] = fips_fatal >> 24;
  U2F_sysinfo.aid[17] = fips_fatal >> 16;
  U2F_sysinfo.aid[18] = fips_fatal >> 8;
  U2F_sysinfo.aid[19] = fips_fatal >> 0;

  /* Sample and cache fips_fatal_lineno, big endian */
  U2F_sysinfo.aid[20] = fips_fatal_lineno >> 24;
  U2F_sysinfo.aid[21] = fips_fatal_lineno >> 16;
  U2F_sysinfo.aid[22] = fips_fatal_lineno >> 8;
  U2F_sysinfo.aid[23] = fips_fatal_lineno >> 0;

  /* Sample and cache sleep_cnt, big endian */
  U2F_sysinfo.aid[24] = sleep_cnt >> 24;
  U2F_sysinfo.aid[25] = sleep_cnt >> 16;
  U2F_sysinfo.aid[26] = sleep_cnt >> 8;
  U2F_sysinfo.aid[27] = sleep_cnt >> 0;

  memcpy(out, &U2F_sysinfo, sizeof(SYSINFO));
  *n = sizeof(SYSINFO);

  return EC_SUCCESS;
}

/* Response CONT frame construction */
static U2FHID_FRAME cont_frame(uint8_t seqno, uint16_t bcnt,
                               uint8_t *buf_resp) {
  U2FHID_FRAME r = {0, {0}};
  int nsent = 57 + seqno * 59;
  int nremaining = bcnt - nsent;
  uint8_t *p = buf_resp + nsent;

  r.cont.seq = seqno;

  if (nremaining >= 59) { /* Full CONT frame */
    memcpy(r.cont.data, p, 59);
  } else {
    memcpy(r.cont.data, p, nremaining);
    memset(r.cont.data + nremaining, 0, 59 - nremaining);
  }

  return r;
}

/* Request complete, send an appropriate response */
/* [ ] P1 TODO: update to handle multi-part response messages by */
/* returning the completed message, then iterating through sending it */
/* in the bottom of the function. Command functions should not call */

/* Response INIT frame construction */
/* TODO: Modify byte count logic to only check in one or a few places
 * if possible. */
static U2FHID_FRAME init_frame(uint8_t cmd, uint16_t bcnt, uint8_t *buf_resp) {
  U2FHID_FRAME r = {0, {0}};
  uint16_t n = bcnt; /* bytes remaining */

  r.init.cmd = cmd;
  r.init.bcntl = 0xFF & bcnt;
  r.init.bcnth = bcnt >> 8;
  /* printf("Copying %d bytes @ %x\n", n, buf_resp); */
  if (n >= 57) {
    /* Full INIT frame */
    memcpy(r.init.data, buf_resp, 57);
  } else {
    memcpy(r.init.data, buf_resp, n);
    memset(r.init.data + n, 0, 57 - n);
  }

  return r;
}

static void u2fhid_response_msg(PENDING_MSG *req) {
  U2FHID_FRAME rsp = {0, {0}};
  uint16_t rsp_len = 0;          // Bytes
  uint8_t num_cont_frames = -1;  // -1 => only INIT frame
  uint8_t i = 0;

  /* Message received */
  cancel_timeout();
  rsp.cid = req->cid;

  /* TRNG may have failed earlier, init *only* when uninitialized.
   * Note we do this lazy late to give the lower level usb handshaking
   * some time to have settled.
   */
  if (fips_fatal == FIPS_UNINITIALIZED) init_fips();

  /* Command dispatch */
  switch (req->cmd | TYPE_MASK) {
  case U2FHID_MSG:
    //printf("Responding to cmd MSG on CID: %02lx\n", req->cid);
      u2fhid_cmd_msg(req->data, req->bcnt, tx_buffer, &rsp_len);
      break;

  case U2FHID_PING:
    //printf("Responding to cmd PING on CID: %02lx\n", req->cid);
    rsp_len = req->bcnt; /* bytes in = bytes out */
    u2fhid_cmd_ping(req->data, req->bcnt, tx_buffer);
    break;

  case U2FHID_LOCK:
    //printf("Responding to cmd LOCK on CID: %02lx\n", req->cid);
    u2fhid_cmd_lock(req->cid, req->data[0]);
    break;

  case U2FHID_WINK:
    //printf("Responding to cmd WINK on CID: %02lx\n", req->cid);
    u2fhid_cmd_wink();
    break;

  case U2FHID_PROMPT:
    //printf("Responding to cmd PROMPT on CID: %02lx\n", req->cid);
    u2fhid_cmd_prompt();
    break;

#if defined(CONFIG_HID_DFU)
    case U2FHID_DFU:
      if (u2fhid_cmd_DFU(req->data, req->bcnt) != EC_SUCCESS) {
        tx_buffer[0] = 99;
        rsp_len = 1;
      }
      break;
#endif

#if defined(CONFIG_FIPS_TEST)
    case U2FHID_FIPS:
      //printf("Responding to U2F command FIPS on CID: %02lx\n", req->cid);
      rsp_len = sizeof(tx_buffer);
      u2fhid_cmd_fips(req->data, req->bcnt, tx_buffer, &rsp_len);
      break;
#endif

  case U2FHID_SYSINFO:
    //printf("Responding to cmd SYSINFO on CID: %02lx\n", req->cid);
    u2fhid_cmd_sysinfo(tx_buffer, &rsp_len);
    break;

    /* TODO: Make this state not-special if possible */
  default:
    //printf("Command %02x on CID %02lx does not exist.\n", req->cmd, req->cid);
    u2fhid_err(req->cid, ERR_INVALID_CMD);
    clear_pending();
    return;
  }

  /* Number of continuation frames needed for response message */
  if (rsp_len > 57)
    num_cont_frames = ((rsp_len - 57) + 58) / 59;
  else
    num_cont_frames = 0;

  /* Construct U2F HID INIT frame */
  rsp = init_frame(req->cmd | TYPE_MASK, rsp_len, tx_buffer);
  rsp.cid = req->cid;
  if (usbu2f_put_frame(&rsp) < 0) goto cleanup;

  /* TODO: Send multiple frames automagically using scatter */
  /* gather. Output-to-host buffer should be 8256B to handle max 129 x */
  /* 64B frames  */

  /* Construct U2F CONT INIT frame(s) */
  for (i = 0; i < num_cont_frames; i++) {
    rsp = cont_frame(i, rsp_len, tx_buffer);
    rsp.cid = req->cid;
    if (usbu2f_put_frame(&rsp) < 0) break;
  }

cleanup:
  /* State's in the pending channel; OK to clear here */
  clear_pending();
}

/* This command synchronizes a channel and optionally requests
 * the device to allocate a unique 32-bit channel identifier
 * (CID) that can be used by the requesting application during
 * its lifetime.
 */
static void u2fhid_cmd_init(U2FHID_FRAME *f_p) {
  U2FHID_FRAME response = {0, .init = {0, 0, 0, {0}}};
  uint32_t proposed_cid;

  /* Create the response packet */
  /* Allocate a new channel if requested from host application */
  if (f_p->cid == CID_BROADCAST) {
    proposed_cid = next_CID++;
    if (next_CID == CID_BROADCAST) next_CID = 1;
    /* Respond on the broadcast channel */
    response.cid = CID_BROADCAST;
  } else {
    proposed_cid = f_p->cid;
    /* Respond on the same channel */
    response.cid = f_p->cid;
  }
  /* Don't even need to check for unallocated CIDs */
  /* TODO (domrizzo): Handle CID rollover, unlikely as it is. */
  /* Init'ing a union inside a structure is a pain */
  response.init.cmd = U2FHID_INIT;
  response.init.bcnth = 0;
  response.init.bcntl = 17;
  /* DATA = 8 B nonce */
  memcpy(&response.init.data, f_p->init.data, 8);
  /* TODO: change 8 to NONCE_SIZE_CONSTANT_NAME */
  /* DATA + 8 = 4 B channel ID */
  response.init.data[INIT_NONCE_SIZE + 0] = (uint8_t)proposed_cid;
  /* N.B. (cast) has higher precedence than shift right */
  response.init.data[INIT_NONCE_SIZE + 1] = (uint8_t)(proposed_cid >> 8);
  response.init.data[INIT_NONCE_SIZE + 2] = (uint8_t)(proposed_cid >> 16);
  response.init.data[INIT_NONCE_SIZE + 3] = (uint8_t)(proposed_cid >> 24);
  /* TODO (domrizzo): Should be constants in a header. */
  /* DATA + 12 = U2FHID protocol version ID */
  response.init.data[12] = U2FHID_IF_VERSION;
  /* DATA + 13 = Major device version num */
  response.init.data[13] = 0;
  /* DATA + 14 = Minor device version num */
  response.init.data[14] = 0;
  /* DATA + 15 = Build device version num */
  response.init.data[15] = 0;
  /* DATA + 16 = Capabilities flags */
  /* TODO: Yes/no? */
  response.init.data[16] = CAPFLAG_WINK | CAPFLAG_LOCK;
  //printf("Response Frame -> cid:%08lx cmd:%02x ", response.cid,
  //        response.init.cmd);
  //printf("bcnth:%02x bcntl:%02x ", response.init.bcnth, response.init.bcntl);
  usbu2f_put_frame(&response);
}

void u2fhid_process_frame(U2FHID_FRAME *f_p);

void u2fhid_process_frame(U2FHID_FRAME *f_p) {
  /* From the U2F HID spec, 2.5.4 Packet sequencing The device keeps
   * track of packets arriving in correct and ascending order and
   * that no expected packets are missing. The device will continue to
   * assemble a message until all parts of it has been received or
   * that the transaction times out. Spurious continuation packets
   * appearing without a prior initialization packet will be
   * ignored.
   */

  uint16_t bcnt = 0;
  //printf("U2F: processing frame at 0x%08x.\n", (unsigned int)f_p);
  /* Channel error checking */
  /* TODO: Would be nice to check anything related to the channel here. */
  /* ERROR: Nothing should ever be on channel 0 */
  if (f_p->cid == 0) {
    printf("No frame should ever use channel 0\n");
    printf("Except this error msg, according to the test. \n");
    u2fhid_err(f_p->cid, ERR_INVALID_CID);
    return;
  }
  /* Only U2FHID_INIT commands on broadcast CID */
  if ((f_p->cid == CID_BROADCAST) &&
      (FRAME_CMD(*f_p) != (~TYPE_MASK & U2FHID_INIT))) {
    printf("Only U2FHID_INIT commands on broadcast CID\n");
    /* TODO: Kosher to respond on the broadcast channel? */
    u2fhid_err(f_p->cid, ERR_INVALID_CID);
    return;
  }

  /* U2FHID_INIT commands are special. Blows through most locks, should respond
   * very quickly. */
  if ((FRAME_TYPE(*f_p) == TYPE_INIT) &&
      (FRAME_CMD(*f_p) == (~TYPE_MASK & U2FHID_INIT))) {
    /* Abort an ongoing multi-packet transaction */
    if (f_p->cid == pending.cid) {
      cancel_timeout();
      clear_pending();
      printf("Pending transaction cancelled\n");
    }

    /* Cope w/ the "special" U2FHID_INIT command */
    //printf("U2F HID Init cmd received\n");
    u2fhid_cmd_init(f_p);
    //printf("U2F HID Init cmd completed\n");
    return;
  }

  /* Normal msg flow; not U2FHID_INIT */
  else {
    /* Channel lock set? */
    if (lock_CID > 0) {
      /* Lock cancels itself on expiration. */
      /* ERR: Other CID attempted to use locked chn */
      if (f_p->cid != lock_CID) {
        printf("Channel locked by U2F_LOCK cmd\n");
        u2fhid_err(f_p->cid, ERR_CHANNEL_BUSY);
        return;
        /* Don't clear the channel */
      }
    }

    /* INIT frame */
    if (FRAME_TYPE(*f_p) == TYPE_INIT) {
      //printf("U2F: Received init frame.\n");
      /* ERROR: Device in use by another channel */
      if ((f_p->cid != pending.cid) && (pending.cid != 0)) {
        printf("U2F: Fob in use by other channel.\n");
        u2fhid_err(f_p->cid, ERR_CHANNEL_BUSY);
        return;
      }
      /* ERROR: Right channel, but CONT frame expected
       */
      if (pending.cid != 0) {
        printf("U2F: Expected CONT frame.\n");
        u2fhid_err(f_p->cid, ERR_INVALID_SEQ);
        /* Clear the channel + timeout */
        cancel_timeout();
        clear_pending();
        return;
      }
      /* ERROR: Message length is too large */
      if (MSG_LEN(*f_p) > MAX_BCNT) {
        printf("U2F: Msg length exceeds max # bytes.\n");
        u2fhid_err(f_p->cid, ERR_INVALID_LEN);
        return;
      }

      /* Init frame through. Begin transaction. */
      /* Start timeout */
      start_timeout(f_p->cid);
      bcnt = MSG_LEN(*f_p);
      /* TODO: Possible to replace this w/ only use of
       * pending struct?
       */
      pending.cid = f_p->cid;
      pending.data = rx_buffer;
      pending.cmd = FRAME_CMD(*f_p);
      pending.seqno = 0;
      pending.bcnt = bcnt;

      /* Singleton or multi-packet request message? */
      /* singleton msg (msg <= 1 frame) */
      if (bcnt <= 57) {
        memcpy(rx_buffer, f_p->init.data, bcnt);
        /* Process response message immediately
         */
        u2fhid_response_msg(&pending);
        /* Clear the channel? */
      }
      /* multi-pkt msg */
      else {
        /* Start filling up the msg buffer */
        memcpy(rx_buffer, f_p->init.data, 57);
      }
      /* INIT frame handled */
      return;
    }
    /* CONTinuation frame */
    else if (FRAME_TYPE(*f_p) == TYPE_CONT) {
      //printf("U2F: Received CONT frame.\n");
      /* ERRORish: No pending transaction, ignore. */
      if (pending.cid == 0 || pending.cid != f_p->cid) {
        printf("U2F: Random CONT packet; ignoring\n");
        return;
      }
      /* ERROR: incorrect sequence # */
      if (pending.seqno != f_p->cont.seq) {
        printf("U2F: Invalid sequence number\n");
        u2fhid_err(f_p->cid, ERR_INVALID_SEQ);
        cancel_timeout();
        clear_pending();
        return;
      }

      /* CONT frame rcv'd w/out error, process */
      /* Restart timeout */
      start_timeout(pending.cid);
      /* Consume frame, process full request msg if last frame */
      if (consume_frame(f_p)) {
        //printf("U2F: Message completed, process.\n");
        u2fhid_response_msg(&pending);
      }
    }
    /* Invalid frame type; shouldn't happen */
    else {
      printf("ERR_OTHER: should never get here.\n");
      printf("frame type: %02x, cmd: %02x\n\n", FRAME_TYPE(*f_p),
              FRAME_CMD(*f_p));
      /* TODO: return ERR_OTHER */
    }
    /* All possible frame types handled */
  }
}

/* Wake up the u2f task to handle a frame */
void u2f_wakeup(void) {
  //  if (task_start_called()) task_set_event(TASK_ID_U2F, TASK_EVENT_FRAME, 0);
}

void u2f_init(void) {
  if (kl_init()) {
    printf("ERROR: kl_init() FAIL!\n");
  }
}

/* N.B. HOOK_INIT happens *before* the initial task scheduling, so you
 * cannot block on TASK_WAKE_EVENT.
 */
/*
static void u2f_task_init(void) {
    const struct SignedHeader *hdr =
  (const struct SignedHeader *)get_program_memory_addr(
          system_get_image_copy());
  const struct SignedHeader *ro_hdr =
    (const struct SignedHeader *)get_program_memory_addr(
          system_get_ro_image_copy());
          int i;

  if (kl_init()) {
    printf("kl_init() FAIL!\n");
  }

  // Clear channel state
  lock_CID = 0;
  timeout_CID = 0;

  // Pad out $BOARD with '_' as id
  for (i = 0; i < sizeof(U2F_sysinfo.id) && BOARD_STRING[i]; ++i)
    U2F_sysinfo.id[i] = BOARD_STRING[i];
  for (; i < sizeof(U2F_sysinfo.id); ++i) U2F_sysinfo.id[i] = '_';

#if defined(CONFIG_SYSINFO_BOARD_ID)
  {
    // Live sample board id pin into the id string
    char c = '0' + board_id();

    if (c > '9') c += 'A' - '0' - 10;
    U2F_sysinfo.id[3] = c;
  }
#endif

  // primary and secondary are equal, since single code blob.
  U2F_sysinfo.primaryMajor = hdr->epoch_;
  U2F_sysinfo.primaryMinor = hdr->major_;
  U2F_sysinfo.primaryBuild = hdr->minor_;
  U2F_sysinfo.secondaryMajor = hdr->epoch_;
  U2F_sysinfo.secondaryMinor = hdr->major_;
  U2F_sysinfo.secondaryBuild = hdr->minor_;
  // put active RO version info and signing key id in AID
  U2F_sysinfo.aidLen = 4 * sizeof(uint32_t);
  memcpy(U2F_sysinfo.aid + 0, &ro_hdr->epoch_, sizeof(uint32_t));
  memcpy(U2F_sysinfo.aid + 4, &ro_hdr->major_, sizeof(uint32_t));
  memcpy(U2F_sysinfo.aid + 8, &ro_hdr->minor_, sizeof(uint32_t));
  memcpy(U2F_sysinfo.aid + 12, &ro_hdr->keyid, sizeof(uint32_t));

  // Append A or B to id so updater can pick right image to update with
  switch (system_get_image_copy()) {
    case (SYSTEM_IMAGE_RW):
      U2F_sysinfo.id[sizeof(U2F_sysinfo.id) - 1] = 'A';
      break;
    case (SYSTEM_IMAGE_RW_B):
      U2F_sysinfo.id[sizeof(U2F_sysinfo.id) - 1] = 'B';
      break;
    default: // Shouldn't happen!
      U2F_sysinfo.id[sizeof(U2F_sysinfo.id) - 1] = '!';
  }
}
*/
//DECLARE_HOOK(HOOK_INIT, u2f_task_init, HOOK_PRIO_DEFAULT);
