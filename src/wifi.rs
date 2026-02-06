use core::mem::ManuallyDrop;

use cyw43::{Control, NetDriver, Runner};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use embassy_rp::{
    Peri,
    gpio::{Level, Output},
    peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0},
    pio::Pio,
};

use static_cell::StaticCell;
#[embassy_executor::task]
pub async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

const FIRMWARE: &[u8] = include_bytes!("../cyw43-firmware/43439A0.bin");
pub const CLM: &[u8] = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

pub struct Wifi {
    pub driver: NetDriver<'static>,
    pub control: Control<'static>,
    pub runner: Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
}

impl Wifi {
    pub async fn init(
        pin_23: Peri<'static, PIN_23>,
        pin_24: Peri<'static, PIN_24>,
        pin_25: Peri<'static, PIN_25>,
        pin_29: Peri<'static, PIN_29>,
        dma_ch0: Peri<'static, DMA_CH0>,
        pio_0: Peri<'static, PIO0>,
    ) -> Self {
        let pwr = Output::new(pin_23, Level::Low);
        let cs = Output::new(pin_25, Level::High);

        let mut pio = ManuallyDrop::new(Pio::new(pio_0, crate::Irqs));
        // SAFETY: we want to leak this, and since everything is 'static, it is safe to do so.
        let mut pio = unsafe { ManuallyDrop::take(&mut pio) };

        let spi = PioSpi::new(
            &mut pio.common,
            pio.sm0,
            DEFAULT_CLOCK_DIVIDER,
            pio.irq0,
            cs,
            pin_24,
            pin_29,
            dma_ch0,
        );

        static WIFI_STATE: StaticCell<cyw43::State> = StaticCell::new();
        let state = WIFI_STATE.init(cyw43::State::new());

        let (driver, control, runner) = cyw43::new(state, pwr, spi, FIRMWARE).await;
        Self {
            driver,
            control,
            runner,
        }
    }
}
