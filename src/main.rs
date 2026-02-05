#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::peripherals::{
    PIN_4, PIN_5, PIN_6, PIN_7, PIN_9, PWM_SLICE2, PWM_SLICE3, PWM_SLICE4,
};
use embassy_rp::pwm::{Pwm, SetDutyCycle};
use embassy_rp::{Peri, pwm};
use embassy_time::{Duration, Timer};

use crate::controller::Controller;
use crate::motor::{Motors, Speed};
use crate::wifi::init_wifi;

use panic_halt as _;
// use {defmt_rtt as _, panic_probe as _};

mod controller;
pub mod motor;
mod wifi;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let (_net_driver, mut net_control, net_runner) =
        init_wifi(p.PIN_23, p.PIN_24, p.PIN_25, p.PIN_29, p.DMA_CH0, p.PIO0).await;
    _ = spawner.spawn(wifi::cyw43_task(net_runner));

    net_control.init(wifi::CLM).await;
    net_control
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

    let delay = Duration::from_secs(1);
    loop {
        // info!("led on!");
        net_control.gpio_set(0, true).await;
        Timer::after(delay).await;

        // info!("led off!");
        net_control.gpio_set(0, false).await;
        Timer::after(delay).await;
    }
}

#[embassy_executor::task]
async fn drive_motors_from_controller(mut motors: Motors, mut controller: Controller) {
    loop {
        motors.drive_speed(Speed::from_percent(1.), Speed::from_percent(1.));
        Timer::after_millis(10).await;
    }
}

// CH1 steering
// CH2 throttle
