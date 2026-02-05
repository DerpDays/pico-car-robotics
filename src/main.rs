#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![no_std]
#![no_main]

use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{
    DMA_CH0, PIN_4, PIN_5, PIN_6, PIN_7, PIN_9, PIN_10, PIO0, PWM_SLICE2, PWM_SLICE3, PWM_SLICE4,
    PWM_SLICE5,
};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pwm::{Pwm, SetDutyCycle};
use embassy_rp::{Peri, bind_interrupts, pwm};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

use panic_halt as _;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    _ = spawner.spawn(cyw43_task(runner));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // Pwm::new_output_a(p.PWM_SLICE0, p.PIN_10, pwm::Config::default());
    // spawner.spawn(pwm_set_dutycycle(p.PWM_SLICE5, p.PIN_10));
    spawner.spawn(drive_motors(
        LeftMotor {
            slice: p.PWM_SLICE2,
            a: p.PIN_4,
            b: p.PIN_5,
        },
        RightMotor {
            slice: p.PWM_SLICE3,
            a: p.PIN_6,
            b: p.PIN_7,
        },
        p.PWM_SLICE4,
        p.PIN_9,
    ));

    let delay = Duration::from_secs(1);
    loop {
        // info!("led on!");
        control.gpio_set(0, true).await;
        Timer::after(delay).await;

        // info!("led off!");
        control.gpio_set(0, false).await;
        Timer::after(delay).await;
    }
}

/// Demonstrate PWM by setting duty cycle
///
/// Using GP4 in Slice2, make sure to use an appropriate resistor.
#[embassy_executor::task]
async fn pwm_set_dutycycle(slice2: Peri<'static, PWM_SLICE5>, pin10: Peri<'static, PIN_10>) {
    // If we aim for a specific frequency, here is how we can calculate the top value.
    // The top value sets the period of the PWM cycle, so a counter goes from 0 to top and then wraps around to 0.
    // Every such wraparound is one PWM cycle. So here is how we get 25KHz:
    let desired_freq_hz = 25_000;
    let clock_freq_hz = embassy_rp::clocks::clk_sys_freq();
    let divider = 16u8;
    let period = (clock_freq_hz / (desired_freq_hz * divider as u32)) as u16 - 1;

    let mut c = pwm::Config::default();
    c.top = period;
    c.divider = divider.into();

    let mut pwm = Pwm::new_output_a(slice2, pin10, c.clone());

    loop {
        // 100% duty cycle, fully on
        pwm.set_duty_cycle_fully_on().unwrap();
        Timer::after_secs(1).await;

        // 66% duty cycle. Expressed as simple percentage.
        pwm.set_duty_cycle_percent(66).unwrap();
        Timer::after_secs(1).await;

        // 25% duty cycle. Expressed as 32768/4 = 8192.
        pwm.set_duty_cycle(c.top / 4).unwrap();
        Timer::after_secs(1).await;

        // 0% duty cycle, fully off.
        pwm.set_duty_cycle_fully_off().unwrap();
        Timer::after_secs(1).await;
    }
}

struct LeftMotor {
    slice: Peri<'static, PWM_SLICE2>,
    a: Peri<'static, PIN_4>,
    b: Peri<'static, PIN_5>,
}
struct RightMotor {
    slice: Peri<'static, PWM_SLICE3>,
    a: Peri<'static, PIN_6>,
    b: Peri<'static, PIN_7>,
}
struct Controller {
    steering: ControllerSteering,
    throtte: ControllerThrottle,
}

struct ControllerSteering {}
struct ControllerThrottle {}
/// Left motor pin 4/5
/// Right motor pin 6/7
#[embassy_executor::task]
async fn drive_motors(
    left_motor: LeftMotor,
    right_motor: RightMotor,
    steering_slice: Peri<'static, PWM_SLICE4>,
    steering_pin: Peri<'static, PIN_9>,
) {
    let desired_freq_hz = 25_000;
    let clock_freq_hz = embassy_rp::clocks::clk_sys_freq();
    let divider = 16u8;
    let period = (clock_freq_hz / (desired_freq_hz * divider as u32)) as u16 - 1;

    let mut c = pwm::Config::default();
    c.top = period;
    c.divider = divider.into();

    let left_motor = Pwm::new_output_ab(left_motor.slice, left_motor.a, left_motor.b, c.clone());
    let right_motor =
        Pwm::new_output_ab(right_motor.slice, right_motor.a, right_motor.b, c.clone());

    let pwm_clock = embassy_rp::clocks::clk_sys_freq();
    let divider: u8 = 16;
    let freq = 100;
    let top = (pwm_clock / (freq * divider as u32)) - 1;
    let mut c = pwm::Config::default();
    c.top = top as u16;
    c.divider = divider.into();
    let steering = Pwm::new_input(
        steering_slice,
        steering_pin,
        embassy_rp::gpio::Pull::None,
        pwm::InputMode::Level,
        c.clone(),
    );

    let (l_a, l_b) = left_motor.split();
    let (mut l_a, mut l_b) = (l_a.unwrap(), l_b.unwrap());

    let (r_a, r_b) = right_motor.split();
    let (mut r_a, mut r_b) = (r_a.unwrap(), r_b.unwrap());

    loop {
        let high = steering.counter() as u32;
        steering.set_counter(0);

        let a = ((high * 100) / (c.top as u32 + 1)).clamp(0, 100) as u8;
        l_a.set_duty_cycle_percent(a).unwrap();
        l_b.set_duty_cycle_fully_off().unwrap();
        r_a.set_duty_cycle_percent(a).unwrap();
        r_b.set_duty_cycle_fully_off().unwrap();
        Timer::after_millis(10).await;
    }
}

// CH1 steering
// CH2 throttle
