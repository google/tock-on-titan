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

#include "personality_syscalls.h"
#include "tock.h"

#define H1B_DRIVER_PERSONALITY 0x5000b

#define TOCK_PERSONALITY_CMD_CHECK   0
#define TOCK_PERSONALITY_CMD_GET     1
#define TOCK_PERSONALITY_CMD_SET     2

#define TOCK_PERSONALITY_ALLOW       0

#define TOCK_PERSONALITY_SET_DONE    0

static void tock_personality_set_done(int unused0 __attribute__((unused)),
                                      int unused1 __attribute__((unused)),
                                      int unused2 __attribute__((unused)),
                                      void *callback_args) {
  *(bool*)callback_args = true;
}

int tock_personality_check(void) {
  return command(H1B_DRIVER_PERSONALITY, TOCK_PERSONALITY_CMD_CHECK, 0, 0);
}

int tock_get_personality(perso_st* personality) {
  int ret = -1;

  ret = allow(H1B_DRIVER_PERSONALITY, TOCK_PERSONALITY_ALLOW,
              personality, sizeof(perso_st));
  if (ret < 0) {
    printf("Could not give kernel access to personality buffer.\n");
    return ret;
  }

  ret = command(H1B_DRIVER_PERSONALITY, TOCK_PERSONALITY_CMD_GET,
                0, 0);
  if (ret < 0) {
    printf("Could not get H1B personality from kernel.\n");
    return ret;
  }

  return TOCK_SUCCESS;
}

int tock_set_personality(const perso_st* personality) {
  int ret = 0;
  bool set_done = false;
  ret = subscribe(H1B_DRIVER_PERSONALITY, TOCK_PERSONALITY_SET_DONE,
                  tock_personality_set_done, &set_done);
  if (ret < 0) {
    printf("Could not register for personality set done callback.\n");
    return ret;
  }

  ret = allow(H1B_DRIVER_PERSONALITY, TOCK_PERSONALITY_ALLOW,
              (perso_st*)personality, sizeof(perso_st));
  if (ret < 0) {
    printf("Could not give kernel access to personality buffer.\n");
    return ret;
  }

  ret = command(H1B_DRIVER_PERSONALITY, TOCK_PERSONALITY_CMD_SET,
                0, 0);
  if (ret < 0) {
    printf("Could not get H1B personality from kernel.\n");
    return ret;
  }
  //yield_for(&set_done);

  return TOCK_SUCCESS;
}
