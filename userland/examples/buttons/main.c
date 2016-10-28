// \file
// This program waits for button presses on each of the buttons attached
// to a board and toggles the LED with the same index. For example, if the first
// button is pressed, the first LED is toggled. If the third button is pressed,
// the third LED is toggled.

#include <button.h>
#include <led.h>

// Callback for button presses.
//   btn_num: The index of the button associated with the callback
//   val: 0 if pressed, 1 if depressed
static void button_callback(int btn_num, int val, int arg2, void *ud) {
  if (val == 0) {
    led_toggle(btn_num);
  }
}

int main(void) {
  button_subscribe(button_callback, NULL);

  // Enable interrupts on each button successively until we run into a button
  // that doesn't exist (negative return value).
  int j = 0;
  for (int i = 0; j >= 0; i++) {
    j = button_enable_interrupt(i);
  }

  return 0;
}

