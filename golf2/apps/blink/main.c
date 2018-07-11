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

