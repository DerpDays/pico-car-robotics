use embassy_rp::Peri;
use embassy_rp::peripherals::{PIN_9, PIN_11, PWM_SLICE4, PWM_SLICE5};
use embassy_rp::pwm::Pwm;

use crate::motor::Speed;

pub struct Controller {
    steering: Steering,
    throttle: Throttle,
}

type SteeringPeripherals = (Peri<'static, PWM_SLICE4>, Peri<'static, PIN_9>);
type ThrottlePeripherals = (Peri<'static, PWM_SLICE5>, Peri<'static, PIN_11>);

impl Controller {
    pub fn init(steering: SteeringPeripherals, throttle: ThrottlePeripherals) -> Self {
        Self {
            steering: Steering::init(steering.0, steering.1),
            throttle: Throttle::init(throttle.0, throttle.1),
        }
    }
    pub fn get_throttle(&self) -> Speed {
        self.throttle.get()
    }
    pub fn get_steering(&self) -> Speed {
        self.throttle.get()
    }
}

struct Steering(Pwm<'static>);
impl Steering {
    fn init(pwm_slice: Peri<'static, PWM_SLICE4>, pwm_pin: Peri<'static, PIN_9>) -> Self {
        let pwm_clock = embassy_rp::clocks::clk_sys_freq();
        let divider: u8 = 16;
        let freq = 100;
        let top = (pwm_clock / (freq * divider as u32)) - 1;
        let mut c = embassy_rp::pwm::Config::default();
        c.top = top as u16;
        c.divider = divider.into();
        let steering = Pwm::new_input(
            pwm_slice,
            pwm_pin,
            embassy_rp::gpio::Pull::None,
            embassy_rp::pwm::InputMode::Level,
            c.clone(),
        );
        Steering(steering)
    }
    fn map_steering(pulse_us: i32) -> f32 {
        const MIN_US: i32 = 1000;
        const MID_US: i32 = 1500;
        const MAX_US: i32 = 2000;

        if pulse_us <= MID_US {
            // Map [MIN .. MID] → [-1.0 .. 0.0]
            (pulse_us - MID_US) as f32 / (MID_US - MIN_US) as f32
        } else {
            // Map [MID .. MAX] → [0.0 .. 1.0]
            (pulse_us - MID_US) as f32 / (MAX_US - MID_US) as f32
        }
    }
    /// Get the steering value normalised to:
    /// left: -1f32
    /// right: -1f32
    fn get(&self) -> f32 {
        let pwm_clock = embassy_rp::clocks::clk_sys_freq();
        let divider: u32 = 16;

        let ticks = self.0.counter() as u32;
        let pulse_us = (ticks * divider * 1_000_000) / pwm_clock;

        let pulse_us = pulse_us as i32;

        Self::map_steering(pulse_us as i32).clamp(-1.0, 1.0)
    }
}
struct Throttle(Pwm<'static>);
impl Throttle {
    fn init(pwm_slice: Peri<'static, PWM_SLICE5>, pwm_pin: Peri<'static, PIN_11>) -> Self {
        let pwm_clock = embassy_rp::clocks::clk_sys_freq();
        let divider: u8 = 16;
        let freq = 100;
        let top = (pwm_clock / (freq * divider as u32)) - 1;
        let mut c = embassy_rp::pwm::Config::default();
        c.top = top as u16;
        c.divider = divider.into();
        let throttle = Pwm::new_input(
            pwm_slice,
            pwm_pin,
            embassy_rp::gpio::Pull::None,
            embassy_rp::pwm::InputMode::Level,
            c.clone(),
        );
        Throttle(throttle)
    }

    fn map_throttle(pulse_us: i32) -> f32 {
        const MIN_US: i32 = 1000;
        const MID_US: i32 = 1500;
        const MAX_US: i32 = 2000;

        if pulse_us <= MID_US {
            // Map [MIN .. MID] → [-1.0 .. 0.0]
            (pulse_us - MID_US) as f32 / (MID_US - MIN_US) as f32
        } else {
            // Map [MID .. MAX] → [0.0 .. 1.0]
            (pulse_us - MID_US) as f32 / (MAX_US - MID_US) as f32
        }
    }

    /// Get the speed of the throttle
    fn get(&self) -> Speed {
        let pwm_clock = embassy_rp::clocks::clk_sys_freq();
        let divider: u32 = 16;

        let ticks = self.0.counter() as u32;
        let pulse_us = (ticks * divider * 1_000_000) / pwm_clock;

        let pulse_us = pulse_us as i32;

        Speed::from_percent(Self::map_throttle(pulse_us as i32))
    }
}
