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

#include <firestorm.h>
#include <stdint.h>
#include <stdio.h>
#include <tock.h>

struct rng_data {
  bool done;
  int len;
};

static void rng_cb( int len,
                    __attribute__ ((unused)) int unused0,
                    __attribute__ ((unused)) int unused1,
                    void* ud) {
  struct rng_data* data = (struct rng_data*) ud;
  data->len = len;
  data->done = true;
}

static int get_random(void *buf, int len) {
  int err = allow(5, 0, buf, len);
  if (err < 0) {
      return err;
  }

  struct rng_data data = { false, 0 };

  err = subscribe(5, 0, rng_cb, &data);
  if (err < 0) {
    return err;
  }

  err = command(5, 0, 0);
  if (err < 0) {
    return err;
  }

  yield_for(&data.done);

  return data.len;
}

int main(void) {
  printf("Hello from the RNG application!\n");

  char buf[1000];
  int len = get_random(buf, sizeof(buf));
  while (len > 0) {
    printf("Read %d bytes of random data.\n", len);
    printf("Sample data = 0x%08lx\n", *(unsigned long *)buf);
    delay_ms(1000);
    len = get_random(buf, sizeof(buf));
  }

  return 0;
}
