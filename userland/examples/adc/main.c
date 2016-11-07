#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

#include "tock.h"
#include "console.h"
#include "timer.h"
#include "adc.h"


int main() {
  putstr("[Tock] ADC Test\n");

  // Setup the ADC
  adc_initialize();
  delay_ms(1000);

  while (1) {
    char buf[128];

    // Sample channel 1. (On Firestorm, this is labeled "A5".)
    int reading = adc_read_single_sample(1);

    // 12 bit, reference = VCC/2, gain = 0.5
    // millivolts = ((reading * 2) / (2^12 - 1)) * (3.3 V / 2) * 1000
    int millivolts = (reading * 3300) / 4095;

    sprintf(buf, "ADC Reading: %i mV (raw: 0x%04x)\n", millivolts, reading);
    putstr(buf);
    delay_ms(1000);
  }

  return 0;
}
