/* vim: set sw=2 expandtab tw=80: */

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

#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

#include <tock.h>
#include <gpio.h>

#define LED_0 0

// callback for timers
timer_cb (int arg0, int arg2, int arg3, void* userdata) {
}

//**************************************************
// GPIO output example
//**************************************************
void gpio_output() {
  putstr("Periodically blinking LED pin\n");

  // set LED pin as output and start repeating timer
  gpio_enable_output(LED_0);
  timer_subscribe(timer_cb, NULL);
  timer_start_repeating(500);

  while (1) {
    gpio_toggle(LED_0);
    wait();
  }
}

//**************************************************
// GPIO input example
//**************************************************
void gpio_input() {
  putstr("Periodically reading value of the LED pin\n");
  putstr("Jump pin high to test (defaults to low)\n");

  // set LED pin as input and start repeating timer
  // pin is configured with a pull-down resistor, so it should read 0 as default
  gpio_enable_input(LED_0, PullDown);
  timer_subscribe(timer_cb, NULL);
  timer_start_repeating(500);

  while (1) {
    // print pin value
    int pin_val = gpio_read(LED_0);
    {
      char buf[64];
      sprintf(buf, "\tValue(%d)\n", pin_val);
      putstr(buf);
    }
    wait();
  }
}

//**************************************************
// GPIO interrupt example
//**************************************************
void gpio_cb (int pin_num, int arg2, int arg3, void* userdata) {
}

void gpio_interrupt() {
  putstr("Print LED pin reading whenever its value changes\n");
  putstr("Jump pin high to test\n");

  // set callback for GPIO interrupts
  gpio_interrupt_callback(gpio_cb, NULL);

  // set LED as input and enable interrupts on it
  gpio_enable_input(LED_0, PullDown);
  gpio_enable_interrupt(LED_0, Change);

  while (1) {
    wait();
    putstr("\tGPIO Interrupt!\n");
  }
}


void main() {
  putstr("*********************\n");
  putstr("GPIO Test Application\n");

  // uncomment whichever example you want
  //gpio_output();
  //gpio_input();
  gpio_interrupt();
}

