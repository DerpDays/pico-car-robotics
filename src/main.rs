#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use crate::controller::Controller;
use crate::motor::Motors;
use crate::wifi::Wifi;

use {defmt_rtt as _, panic_probe as _};

mod controller;

mod display;
pub mod motor;
mod wifi;

embassy_rp::bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<embassy_rp::peripherals::PIO0>;
    I2C0_IRQ => embassy_rp::i2c::InterruptHandler<embassy_rp::peripherals::I2C0>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut wifi_s = Wifi::init(p.PIN_23, p.PIN_24, p.PIN_25, p.PIN_29, p.DMA_CH0, p.PIO0).await;
    spawner.spawn(wifi::cyw43_task(wifi_s.runner)).unwrap();

    wifi_s.control.init(wifi::CLM).await;
    wifi_s
        .control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    spawner
        .spawn(drive_motors_from_controller(
            Motors::init(
                (p.PWM_SLICE2, p.PIN_4, p.PIN_5),
                (p.PWM_SLICE3, p.PIN_6, p.PIN_7),
            ),
            Controller::init(p.PIN_9, p.PIN_11),
        ))
        .expect("failed to spawn motor driver");

    if let Ok(display) = display::Display::new((p.I2C0, p.PIN_1, p.PIN_0)).await {
        spawner
            .spawn(display::drive_display(display))
            .expect("failed to spawn display driver");
    }

    let delay = Duration::from_millis(10000);
    loop {
        // wifi_s.control.gpio_set(0, true).await;
        Timer::after(delay).await;
    }
    //
    // wifi_s.control.gpio_set(0, false).await;
    // Timer::after(delay).await;
    // }
}

#[embassy_executor::task]
async fn drive_motors_from_controller(mut motors: Motors, mut controller: Controller) {
    loop {
        let speed = controller.get_throttle().await;
        motors.drive_speed(speed, speed);
    }
}
