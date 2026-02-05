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

#[allow(unused)]
pub struct Wifi {
    pub driver: NetDriver<'static>,
    pub control: Control<'static>,
    pub runner: Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,

    // rest of the Pio fields.
    common: embassy_rp::pio::Common<'static, PIO0>,
    irq_flags: embassy_rp::pio::IrqFlags<'static, PIO0>,
    irq1: embassy_rp::pio::Irq<'static, PIO0, 1>,
    irq2: embassy_rp::pio::Irq<'static, PIO0, 2>,
    irq3: embassy_rp::pio::Irq<'static, PIO0, 3>,
    sm1: embassy_rp::pio::StateMachine<'static, PIO0, 1>,
    sm2: embassy_rp::pio::StateMachine<'static, PIO0, 2>,
    sm3: embassy_rp::pio::StateMachine<'static, PIO0, 3>,
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
        let pio = Pio::new(pio_0, crate::Irqs);

        let Pio {
            mut common,
            irq_flags,
            irq0,
            irq1,
            irq2,
            irq3,
            sm0,
            sm1,
            sm2,
            sm3,
            ..
        } = pio;

        let spi = PioSpi::new(
            &mut common,
            sm0,
            DEFAULT_CLOCK_DIVIDER,
            irq0,
            cs,
            pin_24,
            pin_29,
            dma_ch0,
        );

        static STATE: StaticCell<cyw43::State> = StaticCell::new();
        let state = STATE.init(cyw43::State::new());

        let (driver, control, runner) = cyw43::new(state, pwr, spi, FIRMWARE).await;
        Self {
            driver,
            control,
            runner,

            common,
            irq_flags,
            irq1,
            irq2,
            irq3,
            sm1,
            sm2,
            sm3,
        }
    }
}
