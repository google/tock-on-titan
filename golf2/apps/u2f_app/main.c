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

// This code is a rewrite/port of the u2f_transport.c code

#include <stdio.h>
#include <string.h>
#include <timer.h>
#include <rng.h>

#define U2F_FRAME_SIZE 64

#include "include/fips.h"
#include "include/storage.h"
#include "include/u2f_syscalls.h"
#include "include/u2f_hid.h"


static void check_device_setup(void);
static void process_frame(U2FHID_FRAME* frame);

static void setup_personality(void);

const perso_st* get_personality(void) {return NULL;}
int check_personality(const perso_st* id) {return id != NULL;}
int new_personality(perso_st* id) {
  if (id == NULL) {
    return 0;
  } else {
    return 1;
  }
}

int set_personality(const perso_st* id) {
  if (id == NULL) {
    return 0;
  } else {
    return 1;
  }
}

static void setup_personality(void) {
  perso_st me;
  if (check_personality(get_personality()) == 1) return;
  if (new_personality(&me) == 1) set_personality(&me);
  printf("    - Personality configured\n");
}


static void check_device_setup(void) {
  //perso_st me;
  printf("  - Checking setup\n");
  ensure_factory_entropy();
  setup_personality();
}

void u2fhid_process_frame(U2FHID_FRAME *f_p);

void process_frame(U2FHID_FRAME* frame) {
  printf("u2f_app: processing frame\n");
  u2fhid_process_frame(frame);
}

int main(void) {
  int ret = 0;

  delay_ms(2000);
  printf("= Running U2F Transport Application =\n");
  delay_ms(100);
  check_device_setup();

  char u2f_buffer[U2F_FRAME_SIZE];
  while (1) {
    printf("u2f_app: receiving frame.\n");
    ret = tock_u2f_receive(u2f_buffer, U2F_FRAME_SIZE);
    U2FHID_FRAME* frame = (U2FHID_FRAME*)u2f_buffer;
    process_frame(frame);
  }
  return ret;
}
