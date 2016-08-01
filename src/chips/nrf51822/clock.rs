use common::VolatileCell;
use common::take_cell::TakeCell;
use core::mem;

struct Registers {
    pub tasks_hfclkstart    : VolatileCell<u32>,
    pub tasks_hfclkstop     : VolatileCell<u32>,
    pub tasks_lfclkstart    : VolatileCell<u32>,
    pub tasks_lfclkstop     : VolatileCell<u32>,
    pub tasks_cal           : VolatileCell<u32>,
    pub tasks_cstart        : VolatileCell<u32>,
    pub tasks_cstop         : VolatileCell<u32>,
    _reserved1: [VolatileCell<u32>; 57],
    pub events_hfclkstarted : VolatileCell<u32>,
    pub events_lfclkstarted : VolatileCell<u32>,
    pub done                : VolatileCell<u32>,
    pub ctto                : VolatileCell<u32>,
    _reserved2: [VolatileCell<u32>; 125],
    pub intenset            : VolatileCell<u32>,
    pub intenclr            : VolatileCell<u32>,
    _reserved3: [VolatileCell<u32>; 63],
    pub hfclkrun            : VolatileCell<u32>,
    pub hfclkstat           : VolatileCell<u32>,
    _reserved4: [VolatileCell<u32>; 1],
    pub lfclkrun            : VolatileCell<u32>,
    pub lfclkstat           : VolatileCell<u32>,
    pub lfclksrccopy        : VolatileCell<u32>,
    _reserved5: [VolatileCell<u32>; 62],
    pub lfclksrc            : VolatileCell<u32>,
    _reserved6: [VolatileCell<u32>; 7],
    pub ctiv                : VolatileCell<u32>,
    _reserved7: [VolatileCell<u32>; 5],
    pub xtalfreq            : VolatileCell<u32>,
}

const CLOCK_BASE: usize = 0x40000000;


pub enum InterruptField {
    HFCLKSTARTED = (1 << 0),
    LFCLKSTARTED = (1 << 1),
    DONE         = (1 << 3),
    CTTO         = (1 << 4),
}

pub enum ClockTaskTriggered {
    NO   = 0,
    YES  = 1,
}

pub enum ClockRunning {
    NORUN   = 0,
    RUN     = (1 << 16),
}

pub enum LowClockSource {
    RC            = 0,
    XTAL          = 1,
    SYNTH         = 2,
    MASK          = 0x3,
}

pub enum HighClockSource {
    RC            = 0,
    XTAL          = 1,
}

pub enum XtalFreq {
    F16MHz         = 0xFF,
    F32MHz         = 0,
}

pub struct Clock {
    client: TakeCell<&'static ClockClient>,
}

pub trait ClockClient {
    // All clock interrupts are control signals, e.g., when
    // a clock has started etc. We don't actually handle any
    // of them for now, but keep this trait in place for if we
    // do need to in the future.
    fn event(&self);
}

pub static mut CLOCK : Clock = Clock {
    client: TakeCell::empty(),
};


impl Clock {
    #[allow(non_snake_case)]
    fn CLOCK(&self) -> &'static Registers {
        unsafe { mem::transmute(CLOCK_BASE as usize) }
    }

    pub fn set_client(&self, client: &'static ClockClient) {
        self.client.replace(client);
    }

    pub fn interrupt_enable(&self, interrupt: InterruptField) {
        self.CLOCK().intenset.set(interrupt as u32);
    }

    pub fn interrupt_disable(&self, interrupt: InterruptField) {
        self.CLOCK().intenclr.set(interrupt as u32);
    }

    pub fn high_start(&self) {
        self.CLOCK().tasks_hfclkstart.set(1);
    }

    pub fn high_stop(&self) {
        self.CLOCK().tasks_hfclkstop.set(1);
    }

    pub fn high_started(&self) -> bool {
        self.CLOCK().events_hfclkstarted.get() == 1
    }

    pub fn high_source(&self) -> HighClockSource {
        match self.CLOCK().hfclkstat.get() & 1 {
            0b0   => HighClockSource::RC,
            _     => HighClockSource::XTAL,
        }
    }

    pub fn high_freq(&self) -> XtalFreq {
        match self.CLOCK().xtalfreq.get() {
            0xff => XtalFreq::F16MHz,
            _    => XtalFreq::F32MHz,
        }
    }

    pub fn high_set_freq(&self, freq: XtalFreq) {
        self.CLOCK().xtalfreq.set(freq as u32);
    }

    pub fn high_running(&self) -> bool {
        (self.CLOCK().hfclkstat.get() & ClockRunning::RUN as u32) == 
            ClockRunning::RUN as u32
    }

    pub fn low_start(&self) {
        self.CLOCK().tasks_lfclkstart.set(1);
    }

    pub fn low_stop(&self) {
        self.CLOCK().tasks_lfclkstop.set(1);
    }

    pub fn low_started(&self) -> bool {
        self.CLOCK().events_lfclkstarted.get() == 1
    }

    pub fn low_source(&self) -> LowClockSource {
        match self.CLOCK().lfclkstat.get() & (LowClockSource::MASK as u32) {
            0b1    => LowClockSource::XTAL,
            0b10   => LowClockSource::SYNTH,
            _ => LowClockSource::RC
        }
    }

    pub fn low_running(&self) -> bool {
        (self.CLOCK().lfclkstat.get() & ClockRunning::RUN as u32) == 
            ClockRunning::RUN as u32
    }

    pub fn low_set_source(&self, src: LowClockSource) {
        self.CLOCK().lfclksrc.set(src as u32);
    
    }
}
