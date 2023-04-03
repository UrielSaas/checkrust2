// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Component for random number generator using `Entropy32ToRandom`.
//!
//! This provides one Component, RngComponent, which implements a userspace
//! syscall interface to the RNG peripheral (TRNG).
//!
//! Usage
//! -----
//! ```rust
//! let rng = components::rng::RngComponent::new(board_kernel, &sam4l::trng::TRNG)
//!     .finalize(rng_component_static!());
//! ```

// Author: Hudson Ayers <hayers@cs.stanford.edu>
// Last modified: 07/12/2019

use capsules_core::rng;
use core::mem::MaybeUninit;
use kernel::capabilities::{Capability, MemoryAllocation};
use kernel::component::Component;
use kernel::hil::entropy::Entropy32;
use kernel::hil::rng::Rng;

#[macro_export]
macro_rules! rng_component_static {
    () => {{
        let etr = kernel::static_buf!(capsules_core::rng::Entropy32ToRandom<'static>);
        let rng = kernel::static_buf!(capsules_core::rng::RngDriver<'static>);

        (etr, rng)
    };};
}

pub struct RngComponent {
    board_kernel: &'static kernel::Kernel,
    driver_num: usize,
    trng: &'static dyn Entropy32<'static>,
}

impl RngComponent {
    pub fn new(
        board_kernel: &'static kernel::Kernel,
        driver_num: usize,
        trng: &'static dyn Entropy32<'static>,
    ) -> RngComponent {
        RngComponent {
            board_kernel: board_kernel,
            driver_num: driver_num,
            trng: trng,
        }
    }
}

impl Component for RngComponent {
    type StaticInput = (
        &'static mut MaybeUninit<capsules_core::rng::Entropy32ToRandom<'static>>,
        &'static mut MaybeUninit<capsules_core::rng::RngDriver<'static>>,
    );
    type Output = &'static rng::RngDriver<'static>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let grant_cap = unsafe { Capability::<MemoryAllocation>::new() };

        let entropy_to_random = static_buffer
            .0
            .write(rng::Entropy32ToRandom::new(self.trng));
        let rng = static_buffer.1.write(rng::RngDriver::new(
            entropy_to_random,
            self.board_kernel.create_grant(self.driver_num, &grant_cap),
        ));
        self.trng.set_client(entropy_to_random);
        entropy_to_random.set_client(rng);

        rng
    }
}
