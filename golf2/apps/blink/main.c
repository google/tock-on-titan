#include <gpio.h>

int main(void) {
  gpio_enable_output(0);
  gpio_enable_output(1);

    while(1) {
      gpio_toggle(0);
      gpio_toggle(1);
      int x = 0;
      for (int i = 0; i < 1000000; i++) {
	x = x + 1;
      }
    }
}

