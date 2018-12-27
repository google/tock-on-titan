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

#include <tock.h>
#include "include/u2f_syscalls.h"


static void tock_u2f_transmit_done(int error __attribute__((unused)),
                                   int fault  __attribute__((unused)),
                                   int unused __attribute__((unused)),
                                   void *callback_args) {
  *(bool*)callback_args = true;
}

int tock_u2f_transmit(void* data, size_t datalen) {
  bool tx_done = false;
  int ret = subscribe(HOTEL_DRIVER_U2F, TOCK_U2F_SUBSCRIBE_TRANSMIT_DONE,
                      tock_u2f_transmit_done, &tx_done);

  if (ret < 0) {
    printf("Could not register U2F transmit callback with kernel: %d\n", ret);
    return ret;
  }

  ret = allow(HOTEL_DRIVER_U2F, TOCK_U2F_ALLOW_TRANSMIT,
              data, datalen);
  if (ret < 0) {
    // This should only occur if application state is not available,
    // which means the driver is busy.
    printf("Could not give kernel access to U2F data: %d\n", ret);
    return TOCK_EBUSY;
  }

  ret = command(HOTEL_DRIVER_U2F, TOCK_U2F_CMD_TRANSMIT, datalen, 0);

  if (ret < 0) {
    printf("Could not transmit over U2F: %d\n", ret);
    return ret;
  }

  yield_for(&tx_done);

  return 0;
}

static void tock_u2f_receive_done(int error __attribute__((unused)),
                                  int fault  __attribute__((unused)),
                                  int unused __attribute__((unused)),
                                  void *callback_args) {
  *(bool*)callback_args = true;
}

int tock_u2f_receive(void* data, size_t datalen) {
  bool rx_done = false;

  int ret = subscribe(HOTEL_DRIVER_U2F, TOCK_U2F_SUBSCRIBE_RECEIVE_DONE,
                      tock_u2f_receive_done, &rx_done);

  if (ret < 0) {
    printf("Could not register U2F receive callback with kernel: %d\n", ret);
    return ret;
  }

  ret = allow(HOTEL_DRIVER_U2F, TOCK_U2F_ALLOW_RECEIVE,
              data, datalen);
  if (ret < 0) {
    // This should only occur if application state is not available,
    // which means the driver is busy.
    printf("Could not give kernel access to U2F data: %d\n", ret);
    return TOCK_EBUSY;
  }

  ret = command(HOTEL_DRIVER_U2F, TOCK_U2F_CMD_RECEIVE, datalen, 0);

  if (ret < 0) {
    printf("Could not receive over U2F: %d\n", ret);
    return ret;
  }

  yield_for(&rx_done);

  return 0;
}
