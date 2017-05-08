#include <stdint.h>
#include <stdio.h>

#include "tock.h"
#include "adc.h"

struct adc_data {
  int reading;
  bool fired;
};

static struct adc_data result = { .fired = false };
static void(*cont_cb)(int);
// Internal callback for faking synchronous reads
static void adc_cb(__attribute__ ((unused)) int callback_type,
                   __attribute__ ((unused)) int channel,
                   int reading,
                   void* ud) {
  struct adc_data* data = (struct adc_data*) ud;
  data->reading = reading;
  data->fired = true;

  // In continuous mode
  if (cont_cb)
      cont_cb(reading);
}

int adc_set_callback(subscribe_cb callback, void* callback_args) {
    return subscribe(DRIVER_NUM_ADC, 0, callback, callback_args);
}

int adc_initialize(void) {
    return command(DRIVER_NUM_ADC, 1, 0);
}

int adc_single_sample(uint8_t channel) {
    return command(DRIVER_NUM_ADC, 2, channel);
}

int adc_cont_sample(uint8_t channel, uint32_t frequency) {
  uint32_t chan_freq = (frequency << 8) | (channel);
  return command(DRIVER_NUM_ADC, 3, chan_freq);
}

int adc_read_single_sample(uint8_t channel) {
  int err;

  cont_cb = NULL;
  result.fired = false;
  err = adc_set_callback(adc_cb, (void*) &result);
  if (err < 0) return err;

  err = adc_single_sample(channel);
  if (err < 0) return err;

  // Wait for the ADC callback.
  yield_for(&result.fired);

  return result.reading;
}

int adc_read_cont_sample(uint8_t channel, uint8_t frequency, void (*cb)(int)) {
  int err;

  cont_cb = cb;
  err = adc_set_callback(adc_cb, (void*) &result);
  if (err < 0) return err;

  err = adc_cont_sample(channel, frequency);

  return err;
}
