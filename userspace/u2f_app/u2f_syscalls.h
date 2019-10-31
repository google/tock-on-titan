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

#ifndef TOCK_U2F_H
#define TOCK_U2F_H

#include <stdlib.h>

// Check whether driver present (0 is success, means present)
int tock_u2f_check(void);
// Transmit as a frame from U2F endpoint. datalen must be <= 64.
int tock_u2f_transmit(void* data, size_t datalen);
// Receive a frame from U2F endopint. datalen must be <= 64.
int tock_u2f_receive(void* data, size_t datalen);

// Low-level chip accesses
uint32_t tock_chip_dev_id0(void);
uint32_t tock_chip_dev_id1(void);
int tock_chip_category(void);

// Robust counter
unsigned int increment_counter(void);

enum touch_state {
  POP_TOUCH_NO  = 0,
  POP_TOUCH_YES = 1,
};

void tock_pop_enable_detection(void);
void tock_pop_set(void);
void tock_pop_clear(void);
enum touch_state tock_pop_check_presence(int consume);

#endif
