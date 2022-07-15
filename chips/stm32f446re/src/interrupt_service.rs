use kernel::deferred_call::DeferredCallManager;
use stm32f4xx::chip::Stm32f4xxDefaultPeripherals;
use stm32f4xx::deferred_calls::DeferredCallTask;

pub struct Stm32f446reDefaultPeripherals<'a> {
    pub stm32f4: Stm32f4xxDefaultPeripherals<'a>,
    // Once implemented, place Stm32f446re specific peripherals here
}

impl<'a> Stm32f446reDefaultPeripherals<'a> {
    pub unsafe fn new(
        rcc: &'a crate::rcc::Rcc,
        exti: &'a crate::exti::Exti<'a>,
        dma1: &'a crate::dma::Dma1<'a>,
        dma2: &'a crate::dma::Dma2<'a>,
        dc_mgr: &'static DeferredCallManager<DeferredCallTask>,
    ) -> Self {
        Self {
            stm32f4: Stm32f4xxDefaultPeripherals::new(rcc, exti, dma1, dma2, dc_mgr),
        }
    }
    // Necessary for setting up circular dependencies
    pub fn init(&'a self) {
        self.stm32f4.setup_circular_deps();
    }
}
impl<'a> kernel::platform::chip::InterruptService<DeferredCallTask>
    for Stm32f446reDefaultPeripherals<'a>
{
    unsafe fn service_interrupt(&self, interrupt: u32) -> bool {
        match interrupt {
            // put Stm32f446re specific interrupts here
            _ => self.stm32f4.service_interrupt(interrupt),
        }
    }
    unsafe fn service_deferred_call(&self, task: DeferredCallTask) -> bool {
        self.stm32f4.service_deferred_call(task)
    }
}
