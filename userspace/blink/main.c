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

#include <gpio.h>
int x;

int main(void) {
  printf("Booted Blink app.\n");
  gpio_enable_output(0);

  while(1) {
    gpio_toggle(0);
    for (int i = 0; i < 1000000; i++) {x+=i;}
  }
}

