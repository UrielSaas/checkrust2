use core::cell::Cell;
use hil::{Driver,Callback};
use hil::i2c::I2C;
use hil::timer::*;

#[allow(dead_code)]
enum Registers {
    SensorVoltage = 0x00,
    LocalTemperature = 0x01,
    Configuration = 0x02,
    ManufacturerID = 0xFE,
    DeviceID = 0xFF
}

pub struct TMP006<'a, I: I2C + 'a> {
    i2c: &'a I,
    timer: &'a Timer,
    last_temp: Cell<Option<i16>>,
    callback: Cell<Option<Callback>>,
    enabled: Cell<bool>
}

impl<'a, I: I2C> TMP006<'a, I> {
    pub fn new(i2c: &'a I, timer: &'a Timer) -> TMP006<'a, I> {
        TMP006{
            i2c: i2c,
            timer: timer,
            last_temp: Cell::new(None),
            callback: Cell::new(None),
            enabled: Cell::new(false)
        }
    }
}

impl<'a, I: I2C> TimerClient for TMP006<'a, I> {
    fn fired(&self, _: u32) {
        let mut buf: [u8; 3] = [0; 3];

        // If not ready, wait for next timer fire
        self.i2c.read_sync(0x40, &mut buf[0..2]);
        if buf[1] & 0x80 != 0x80 {
            return;
        }

        // Now set the correct register pointer value so we can issue a read
        // to the sensor voltage register
        buf[0] = Registers::SensorVoltage as u8;
        self.i2c.write_sync(0x40, &buf[0..1]);

        // Now read the sensor reading
        self.i2c.read_sync(0x40, &mut buf[0..2]);
        //let sensor_voltage = (((buf[0] as u16) << 8) | buf[1] as u16) as i16;

        // Now move the register pointer to the die temp register
        buf[0] = Registers::LocalTemperature as u8;
        self.i2c.write_sync(0x40, &buf[0..1]);

        // Now read the 14bit die temp
        self.i2c.read_sync(0x40, &mut buf[0..2]);
        let die_temp = (((buf[0] as u16) << 8) | buf[1] as u16) as i16;

        // Shift to the right to make it 14 bits (this should be a signed shift)
        // The die temp is is in 1/32 degrees C.
        let final_temp = die_temp >> 2;
        self.last_temp.set(Some(final_temp));
        self.callback.get().map(|mut cb| {
            cb.schedule(final_temp as usize, 0, 0);
        });
    }
}

impl<'a, I: I2C> Driver for TMP006<'a, I> {
    fn subscribe(&self, subscribe_num: usize, mut callback: Callback) -> isize {
        match subscribe_num {
            0 /* read temperature  */ => {
                if !self.enabled.get() {
                    return -1;
                }
                match self.last_temp.get() {
                    Some(temp) => {
                        callback.schedule(temp as usize, 0, 0);
                    },
                    None => {
                        self.callback.set(Some(callback));
                    }
                }
                0
            },
            _ => -1
        }
    }

    fn command(&self, cmd_num: usize, _: usize, _: usize) -> isize {
        match cmd_num {
            0 /* Enable sensor  */ => {
                self.i2c.enable();

                let mut buf: [u8; 3] = [0; 3];

                // Start by enabling the sensor
                let config = 0x7 << 12;
                buf[0] = Registers::Configuration as u8;
                buf[1] = ((config & 0xFF00) >> 8) as u8;
                buf[2] = (config & 0x00FF) as u8;
                self.i2c.write_sync(0x40, &buf);

                self.timer.repeat(32768);

                self.enabled.set(true);

                0
            },
            _ => -1
        }
    }
}

