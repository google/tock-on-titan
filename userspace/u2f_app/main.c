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

// This code is a rewrite/port of the u2f_transport.c code from
// Cr52. It implements a U2F (2 factor authentication token)
// application as a process in the Tock operating system. It is
// intended to run on an H1B tock-on-titan Tock kernel. It depends on
// the following system call drivers:
//   - GPIO (for user button presses)
//   - H1B_DRIVER_DCRYPTO (for ECC crypto acceleration)
//   - H1B_DRIVER_DIGEST (for SHA256 hash acceleration)
//   - AES_DRIVER (the H1B variant in libh1, for AES acceleration)
//   - H1B_DRIVER_U2F (for USB transport to the token over EP1)
//   - CONSOLE (for printing out messages)
//   - RNG (for entropy generation)

#include <stdio.h>
#include <string.h>
#include <timer.h>
#include <rng.h>

#define U2F_FRAME_SIZE 64

#include "fips.h"
#include "kl.h"
#include "storage.h"
#include "u2f_syscalls.h"
#include "u2f_hid.h"
#include "x509.h"

static void check_device_setup(void);
static void process_frame(U2FHID_FRAME* frame);
static void setup_personality(void);

int check_personality(const perso_st* id) {
  uint32_t chksum[8];
  uint32_t err = 0;
  size_t i;
  if (id == NULL) {
    return EC_ERROR_UNKNOWN;
  }
  err = kl_derive_attest(id->cert_hash, chksum);
  for (i = 0; i < 8; ++i) {
    err |= (chksum[i] ^ id->chksum[i]);
  }
  return err == 0 ? EC_SUCCESS : EC_ERROR_UNKNOWN;
}

int new_personality(perso_st* id) {
   p256_int priv;
  LITE_SHA256_CTX ctx;  // SHA256 output container
  int err = 0;

  if (id == NULL) {
    return EC_ERROR_UNKNOWN;
  }

  memset(id, 0xff, sizeof(perso_st));

  err |= kl_random(id->salt);

  err |= individual_keypair(&priv, &id->pub_x, &id->pub_y, id->salt);
  id->cert_len = generate_cert(&priv, &id->pub_x, &id->pub_y, 1, id->cert,
                               sizeof(id->cert));
  SHA256_INIT(&ctx);
  SHA256_UPDATE(&ctx, id->cert, id->cert_len);
  memcpy(id->cert_hash, SHA256_FINAL(&ctx), SHA256_DIGEST_SIZE);

  err |= kl_derive_attest(id->cert_hash, id->chksum);
  //printf("Setting personality\n");
  //set_personality(id);
  //printf("Personality set\n");

  return err == 0 ? EC_SUCCESS : EC_ERROR_UNKNOWN;
}

static void setup_personality(void) {
  perso_st* person = get_personality();
  if (check_personality(person) == EC_SUCCESS) return;
  printf("Personality not found: generating and storing.\n");
  if (new_personality(person) == EC_SUCCESS) set_personality(person);
}


static void check_device_setup(void) {
  printf("Setting up device entropy.\n");
  ensure_factory_entropy();
  printf("Setting up device personality.\n");
  setup_personality();
  printf("Setup complete.\n");
}

void u2fhid_process_frame(U2FHID_FRAME *f_p);

void process_frame(U2FHID_FRAME* frame) {
  //printf("U2F APP: processing frame\n");
  u2fhid_process_frame(frame);
  //printf("U2F APP: completed processing frame\n");
}

char u2f_buffer[U2F_FRAME_SIZE];

int main(void) {
  int ret = 0;
  printf("= Booting U2F application =\n");
  init_fips();

  if (kl_init()) {
    printf("kl_init() FAIL\n");
  }

  tock_pop_enable_detection();

  printf("= Configuring device state and identity = \n");
  check_device_setup();
  u2f_init();
  printf("= Running U2F application =\n");

  while (1) {
    //printf("U2F APP: receiving frame into 0x%08x.\n", (unsigned int)u2f_buffer);
    ret = tock_u2f_receive(u2f_buffer, U2F_FRAME_SIZE);
    if (ret != 0) {
      printf("U2F APP: error %i in receive, retry.\n", ret);
      continue;
    }
    U2FHID_FRAME* frame = (U2FHID_FRAME*)u2f_buffer;
    process_frame(frame);
  }
  return ret;
}
