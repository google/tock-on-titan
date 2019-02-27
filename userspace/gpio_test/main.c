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
#include <timer.h>

#define LED_0 0
#define BUTTON_PIN 1

void output_cb(int arg0, int arg2, int arg3, void* userdata);
void gpio_output(void);
void input_cb(int arg0, int arg2, int arg3, void* userdata);
void gpio_input(void);
void interrupt_cb(int arg0, int arg2, int arg3, void* userdata);
void gpio_interrupt(void);

tock_timer_t timer;

//**************************************************
// GPIO output example
//**************************************************
void output_cb(__attribute__ ((unused)) int arg0,
               __attribute__ ((unused)) int arg1,
               __attribute__ ((unused)) int arg2,
               __attribute__ ((unused)) void* ud) {
  gpio_toggle(LED_0);
}



void gpio_output(void) {
  printf("Periodically blinking LED pin\n");
  // set LED pin as output and start repeating timer
  gpio_enable_output(LED_0);
  timer_every(500, output_cb, NULL, &timer);
}

int counter = 0;
//**************************************************
// GPIO input example
//**************************************************
void input_cb(__attribute__ ((unused)) int arg0,
              __attribute__ ((unused)) int arg1,
              __attribute__ ((unused)) int arg2,
              __attribute__ ((unused)) void* ud) {
  int pin_val = gpio_read(BUTTON_PIN);
  counter++;
  printf("\t[%04x]: Value(%d)\n", counter, pin_val);
}

void gpio_input(void) {
  printf("Periodically reading value of the button pin\n");
  printf("Press button to test\n");

  // set LED pin as input and start repeating timer
  // pin is configured with a pull-down resistor, so it should read 0 as default
  gpio_enable_input(LED_0, PullUp);
  timer_every(500, input_cb, NULL, &timer);
}

//**************************************************
// GPIO interrupt example
//**************************************************
void interrupt_cb(__attribute__ ((unused)) int arg0,
                  __attribute__ ((unused)) int arg1,
                  __attribute__ ((unused)) int arg2,
                  __attribute__ ((unused)) void* ud) {
  printf("\tGPIO interrupt!\n");
}

void gpio_interrupt(void) {
  printf("Print button pin reading whenever its value changes\n");
  printf("Press user button to test\n");

  // set callback for GPIO interrupts
  gpio_interrupt_callback(interrupt_cb, NULL);
  // set LED as input and enable interrupts on it
  gpio_enable_input(BUTTON_PIN, PullUp);
  gpio_enable_interrupt(BUTTON_PIN, Change);
}


int main(void) {
  printf("*********************\n");
  printf("GPIO Test Application\n");

  // uncomment whichever example you want
  //gpio_output();
  //gpio_input();
  gpio_interrupt();
  return 0;
}
