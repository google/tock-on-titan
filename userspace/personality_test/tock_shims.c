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

#include "personality_syscalls.h"
#include "tock.h"
#include "common.h"

void get_personality(perso_st* id) {
  tock_get_personality(id);
}

int set_personality(const perso_st* id) {
  int rval = tock_set_personality(id);
  if (rval == TOCK_SUCCESS) {
    return EC_SUCCESS;
  } else {
    return EC_ERROR_UNKNOWN;
  }
}
