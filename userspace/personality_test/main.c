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

// This tests the personality driver, which persistently stores some
// U2F application state in flash.


#include "common.h"
#include "storage.h"

#include <stdio.h>
#include <string.h>
#include <timer.h>
#include <rng.h>


static perso_st me;

static void setup_personality(void);

int check_personality(const perso_st* id) {
  return id != NULL;
}

int new_personality(perso_st* id) {
  if (id == NULL) {
    return 0;
  } else {
    return 1;
  }
}

static void setup_personality(void) {
  get_personality(&me);
  if (check_personality(&me) == EC_SUCCESS) return;
  if (new_personality(&me) == EC_SUCCESS) set_personality(&me);
  printf("    - Personality configured\n");
}


static void check_device_setup(void) {
  //perso_st me;
  printf("  - Checking setup\n");
  //ensure_factory_entropy();
  printf("  - Setting up personality.\n");
  setup_personality();
  printf("  - Setup complete.\n");
}

int main(void) {
  printf("= Testing Personality Driver =\n");
  check_device_setup();
  return 0;
}
