//! Components for the LSM303DLHC sensor.
//!
//! I2C Interface
//!
//! Usage
//! -----
//! ```rust
//! let lsm303agr = components::lsm303agr::Lsm303agrI2CComponent::new(mux_i2c, None, None, board_kernel, DRIVER_NUM)
//!    .finalize(components::lsm303agr_component_static!());
//!
//! lsm303agr.configure(
//!    lsm303agr::Lsm303dlhcAccelDataRate::DataRate25Hz,
//!    false,
//!    lsm303agr::Lsm303dlhcScale::Scale2G,
//!    false,
//!    true,
//!    lsm303agr::Lsm303dlhcMagnetoDataRate::DataRate3_0Hz,
//!    lsm303agr::Lsm303dlhcRange::Range4_7G,
//! );
//! ```

use capsules_core::virtualizers::virtual_i2c::{I2CDevice, MuxI2C};
use capsules_extra::lsm303agr::Lsm303agrI2C;
use capsules_extra::lsm303xx;
use core::mem::MaybeUninit;
use kernel::capabilities::{Capability, MemoryAllocation};
use kernel::component::Component;

// Setup static space for the objects.
#[macro_export]
macro_rules! lsm303agr_component_static {
    () => {{
        let buffer = kernel::static_buf!([u8; 8]);
        let accelerometer_i2c =
            kernel::static_buf!(capsules_core::virtualizers::virtual_i2c::I2CDevice);
        let magnetometer_i2c =
            kernel::static_buf!(capsules_core::virtualizers::virtual_i2c::I2CDevice);
        let lsm303agr = kernel::static_buf!(capsules_extra::lsm303agr::Lsm303agrI2C<'static>);

        (accelerometer_i2c, magnetometer_i2c, buffer, lsm303agr)
    };};
}

pub struct Lsm303agrI2CComponent {
    i2c_mux: &'static MuxI2C<'static>,
    accelerometer_i2c_address: u8,
    magnetometer_i2c_address: u8,
    board_kernel: &'static kernel::Kernel,
    driver_num: usize,
}

impl Lsm303agrI2CComponent {
    pub fn new(
        i2c_mux: &'static MuxI2C<'static>,
        accelerometer_i2c_address: Option<u8>,
        magnetometer_i2c_address: Option<u8>,
        board_kernel: &'static kernel::Kernel,
        driver_num: usize,
    ) -> Lsm303agrI2CComponent {
        Lsm303agrI2CComponent {
            i2c_mux,
            accelerometer_i2c_address: accelerometer_i2c_address
                .unwrap_or(lsm303xx::ACCELEROMETER_BASE_ADDRESS),
            magnetometer_i2c_address: magnetometer_i2c_address
                .unwrap_or(lsm303xx::MAGNETOMETER_BASE_ADDRESS),
            board_kernel,
            driver_num,
        }
    }
}

impl Component for Lsm303agrI2CComponent {
    type StaticInput = (
        &'static mut MaybeUninit<I2CDevice<'static>>,
        &'static mut MaybeUninit<I2CDevice<'static>>,
        &'static mut MaybeUninit<[u8; 8]>,
        &'static mut MaybeUninit<Lsm303agrI2C<'static>>,
    );
    type Output = &'static Lsm303agrI2C<'static>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let grant_cap = unsafe { Capability::<MemoryAllocation>::new() };

        let buffer = static_buffer.2.write([0; 8]);

        let accelerometer_i2c = static_buffer
            .0
            .write(I2CDevice::new(self.i2c_mux, self.accelerometer_i2c_address));
        let magnetometer_i2c = static_buffer
            .1
            .write(I2CDevice::new(self.i2c_mux, self.magnetometer_i2c_address));

        let grant = self.board_kernel.create_grant(self.driver_num, &grant_cap);
        let lsm303agr = static_buffer.3.write(Lsm303agrI2C::new(
            accelerometer_i2c,
            magnetometer_i2c,
            buffer,
            grant,
        ));
        accelerometer_i2c.set_client(lsm303agr);
        magnetometer_i2c.set_client(lsm303agr);

        lsm303agr
    }
}
