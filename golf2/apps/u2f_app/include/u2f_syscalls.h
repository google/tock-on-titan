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

#define HOTEL_DRIVER_U2F 0x20008

#define TOCK_U2F_CMD_CHECK    0

#define TOCK_U2F_CMD_TRANSMIT 1
#define TOCK_U2F_CMD_RECEIVE  2

#define TOCK_U2F_ALLOW_TRANSMIT 1
#define TOCK_U2F_ALLOW_RECEIVE  2

#define TOCK_U2F_SUBSCRIBE_TRANSMIT_DONE 1
#define TOCK_U2F_SUBSCRIBE_RECEIVE_DONE  2
#define TOCK_U2F_SUBSCRIBE_RECONNECT     3

// Transmit as a frame from U2F endpoint. datalen must be <= 64.
int tock_u2f_transmit(void* data, size_t datalen);
// Receive a frame from U2F endopint. datalen must be <= 64.
int tock_u2f_receive(void* data, size_t datalen);


// Low-level chip accesses
int tock_chip_dev_id0();
int tock_chip_dev_id1();
int tock_chip_category();

#endif
