# Application Code
This document explains how application code works in Tock. This is not a guide
to creating your own applications, but rather documentation of the design
thoughts behind how applications function.


## Overview of Applications in Tock
Applications in Tock are the user-level code meant to accomplish some type of
task for the end user. Applications are distinguished from kernel code which
handles device drivers, chip-specific details, and general operating system
tasks. Unlike many existing embedded operating systems, in Tock applications
are not built as one with the kernel. Instead they are entirely separate code
that interact with the kernel and each other through [system
calls](https://en.wikipedia.org/wiki/System_call).

Since applications are not a part of the kernel, they may be written in any
language that can be compiled into code capable of running on ARM Cortex-M
processors. While the Tock kernel is written in Rust, applications are commonly
written in C. Additionally, Tock supports running multiple applications concurrently.
Co-operatively multiprogramming is the default, but applications may also be
time sliced. Applications may talk to each other via Inter-Process
Communication (IPC) through system calls.

Applications do not have compile-time knowledge of the address at which they
will be installed and loaded. In the current design of Tock, applications must
be compiled as [position independent
code](https://en.wikipedia.org/wiki/Position-independent_code) (PIC). This
allows them to be run from any address they happen to be loaded into. The use
of PIC for Tock apps is not a fundamental choice, future versions of the system
may support run-time relocatable code.

Applications are unprivileged code. They may not access all portions of memory
and may, in fact, fault if they attempt to access memory outside of their
boundaries (similarly to segmentation faults in Linux code). In order to
interact with hardware, applications must make calls to the kernel.


## System Calls
System calls (aka syscalls) are used to send commands to the kernel. These
could include commands to drivers, subscriptions to callbacks, granting of
memory to the kernel to use to store received data, communication with other
application code, and many others. In practice, the system call is taken care
of by library code and the application need not deal with them directly.

For example the following is the system call handling the `gpio_set` command
from [gpio.c](../userland/libtock/gpio.c):

```c
int gpio_set(GPIO_Pin_t pin) {
  return command(GPIO_DRIVER_NUM, 1, pin);
}
```

The command system call itself is implemented as the ARM assembly instruction
`svc` (service call) in [tock.c](../userland/libtock/tock.c):

```c
int __attribute__((naked))
command(uint32_t driver, uint32_t command, int data) {
  asm volatile("svc 2\nbx lr" ::: "memory", "r0");
}
```

A more in-depth discussion of can be found in the [system call documentation](./Syscalls.md).


## Callbacks
Tock is designed to support embedded applications, which often handle
asynchronous events through the use of [callback
functions](https://en.wikipedia.org/wiki/Callback_(computer_programming)). For
example, in order to receive timer callbacks, you first call `timer_subscribe`
with a function pointer to your own function that you want called when the
timer fires. Specific state that you want the callback to act upon can be
passed as the pointer `userdata`. After the application has started the timer,
calls `yield`, and the timer fires, the callback function will be called.

It is important to note that `yield` must be called in order for events to be
serviced in the current implementation of Tock. Callbacks to the application
will be queued when they occur but the application will not receive them until
it yields. This is not fundamental to Tock, and future version may service
callbacks on any system call or when performing application time slicing. After
receiving and running the callback, application code will continue after the
`yield`. Tock automatically calls `yield` continuously for applications that
return from execution (for example, an application that returns from `main`).


## Inter-Process Communication
 * how does this work?


## Libraries
Application code does not need to stand alone, libraries are available that can
be utilized! 

### Newlib
Application code written in C has access to most of the [C standard
library](https://en.wikipedia.org/wiki/C_standard_library) which is implemented
by [Newlib](https://en.wikipedia.org/wiki/Newlib). Newlib is focused on
providing capabilities for embedded systems. It provides interfaces such as
`printf`, `malloc`, and `memcpy`. Most, but not all features of the standard
library are available to applications. The built configuration of Newlib is
specified in [build.sh](../userland/newlib/build.sh).


### libtock
 * These are the libraries wrapping syscalls

## Tock Binary Format
 * list the header for the top of code
 * talk about rel data section
 * point to elf2tbf rs tool

