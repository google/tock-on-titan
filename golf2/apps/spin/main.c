#include <gpio.h>
int x;

int main(void) {
  while(1) {
    for (int i = 0; i < 1000000; i++) {x+=i;}
  }
}

