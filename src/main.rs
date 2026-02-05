#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Timer;

use crate::controller::Controller;
use crate::motor::{Motors, Speed};
use crate::wifi::Wifi;

// use panic_halt as _;
use {defmt_rtt as _, panic_probe as _};

mod controller;
pub mod motor;
mod wifi;

embassy_rp::bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<embassy_rp::peripherals::PIO0>;
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
            Controller::init((p.PWM_SLICE4, p.PIN_9), (p.PWM_SLICE5, p.PIN_11)),
        ))
        .unwrap();

    let delay = embassy_time::Duration::from_secs(1);
    loop {
        // defmt::info!("led on!");
        wifi_s.control.gpio_set(0, true).await;
        Timer::after(delay).await;

        // defmt::info!("led off!");
        wifi_s.control.gpio_set(0, false).await;
        Timer::after(delay).await;
    }
}

#[embassy_executor::task]
async fn drive_motors_from_controller(mut motors: Motors, mut controller: Controller) {
    let mut a = 0.;
    let mut direction = true;
    loop {
        let speed = if direction {
            if a >= 1. {
                direction = false;
            } else {
                a += 0.01;
            }
            Speed::from_percent(a)
        } else {
            if a <= -1. {
                direction = true;
            } else {
                a -= 0.01;
            }
            Speed::from_percent(a)
        };
        motors.drive_speed(speed, speed);
        Timer::after_millis(10).await;
    }
}

// CH1 steering
// CH2 throttle
