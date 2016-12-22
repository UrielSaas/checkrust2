use platform::{Chip, Platform, SysTick};
use process;
use process::{Process, Task};
use syscall;

pub unsafe fn do_process<P: Platform, C: Chip>(platform: &P,
                                               chip: &mut C,
                                               process: &mut Process,
                                               appid: ::AppId,
                                               ipc: &::ipc::IPC) {
    let systick = chip.systick();
    systick.reset();
    systick.set_timer(10000);
    systick.enable(true);

    loop {
        if chip.has_pending_interrupts() || systick.overflowed() || systick.value() <= 500 {
            break;
        }

        match process.current_state() {
            process::State::Running => {
                process.setup_mpu(chip.mpu());
                systick.enable(true);
                process.switch_to();
                systick.enable(false);
            }
            process::State::Yielded => {
                match process.dequeue_task() {
                    None => break,
                    Some(cb) => {
                        match cb {
                            Task::FunctionCall(ccb) => {
                                process.push_function_call(ccb);
                            }
                            Task::IPC((otherapp, ipc_type)) => {
                                ipc.schedule_callback(appid, otherapp, ipc_type);
                            }
                        }
                        continue;
                    }
                }
            }
            process::State::Fault => {
                //XXX: really need to deal with the app
                panic!("OH NO. Trying to schedule a Faulty app");
            }
        }

        if !process.syscall_fired() {
            break;
        }

        // check if the app had a fault
        if process.app_fault() {
            process.fault_state();
            process.pop_syscall_stack();

            //XXX: Start doing something about it
            panic!("AHHHHH!!! App errored");

            continue
        }

        match process.svc_number() {
            Some(syscall::MEMOP) => {
                let brk_type = process.r0();
                let r1 = process.r1();

                let res = match brk_type {
                    0 /* BRK */ => {
                        process.brk(r1 as *const u8)
                            .map(|_| 0).unwrap_or(-1)
                    },
                    1 /* SBRK */ => {
                        process.sbrk(r1 as isize)
                            .map(|addr| addr as isize).unwrap_or(-1)
                    },
                    _ => -2
                };
                process.set_r0(res);
            }
            Some(syscall::YIELD) => {
                process.yield_state();
                process.pop_syscall_stack();

                // There might be already enqueued callbacks
                continue;
            }
            Some(syscall::SUBSCRIBE) => {
                let driver_num = process.r0();
                let subdriver_num = process.r1();
                let callback_ptr = process.r2() as *mut ();
                let appdata = process.r3();

                let callback = ::Callback::new(appid, appdata, callback_ptr);
                let res = platform.with_driver(driver_num, |driver| {
                    match driver {
                        Some(d) => d.subscribe(subdriver_num, callback),
                        None => -1,
                    }
                });
                process.set_r0(res);
            }
            Some(syscall::COMMAND) => {
                let res = platform.with_driver(process.r0(), |driver| {
                    match driver {
                        Some(d) => d.command(process.r1(), process.r2(), appid),
                        None => -1,
                    }
                });
                process.set_r0(res);
            }
            Some(syscall::ALLOW) => {
                let res = platform.with_driver(process.r0(), |driver| {
                    match driver {
                        Some(d) => {
                            let start_addr = process.r2() as *mut u8;
                            let size = process.r3();
                            if process.in_exposed_bounds(start_addr, size) {
                                let slice = ::AppSlice::new(start_addr as *mut u8, size, appid);
                                d.allow(appid, process.r1(), slice)
                            } else {
                                -1
                            }
                        }
                        None => -1,
                    }
                });
                process.set_r0(res);
            }
            _ => {}
        }
    }
    systick.reset();
}
