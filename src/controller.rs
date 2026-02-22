use embassy_rp::Peri;
use embassy_rp::gpio::Input;
use embassy_rp::peripherals::{PIN_9, PIN_11};
use embassy_time::Instant;

use crate::motor::Speed;

pub struct Controller {
    steering: Steering,
    throttle: Throttle,
}

type SteeringPin = Peri<'static, PIN_9>;
type ThrottlePin = Peri<'static, PIN_11>;

impl Controller {
    pub fn init(steering: SteeringPin, throttle: ThrottlePin) -> Self {
        Self {
            steering: Steering::init(steering),
            throttle: Throttle::init(throttle),
        }
    }
    pub async fn get_throttle(&mut self) -> Speed {
        self.throttle.get().await
    }
    pub async fn get_steering(&mut self) -> f32 {
        self.steering.get().await
    }
}

struct Steering(Input<'static>);
impl Steering {
    fn init(pin: SteeringPin) -> Self {
        Steering(Input::new(pin, embassy_rp::gpio::Pull::None))
    }

    fn map_steering(pulse_us: i32) -> f32 {
        const MIN_US: i32 = 1100;
        const MID_US: i32 = 1450;
        const MAX_US: i32 = 1800;

        if pulse_us <= MID_US {
            // Map [MIN .. MID] → [-1.0 .. 0.0]
            (pulse_us - MID_US) as f32 / (MID_US - MIN_US) as f32
        } else {
            // Map [MID .. MAX] → [0.0 .. 1.0]
            (pulse_us - MID_US) as f32 / (MAX_US - MID_US) as f32
        }
    }

    pub async fn get(&mut self) -> f32 {
        self.0.wait_for_rising_edge().await;
        let start = Instant::now();
        self.0.wait_for_falling_edge().await;
        let duration = Instant::now().duration_since(start);

        let pulse_us = duration.as_micros() as i32;

        Self::map_steering(pulse_us)
    }
}
struct Throttle(Input<'static>);
impl Throttle {
    fn init(pin: ThrottlePin) -> Self {
        Throttle(Input::new(pin, embassy_rp::gpio::Pull::None))
    }

    fn map_throttle(pulse_us: i32) -> f32 {
        const MIN_US: i32 = 1050;
        const MID_US: i32 = 1400;
        const MAX_US: i32 = 1850;

        if pulse_us <= MID_US {
            // Map [MIN .. MID] → [-1.0 .. 0.0]
            (pulse_us - MID_US) as f32 / (MID_US - MIN_US) as f32
        } else {
            // Map [MID .. MAX] → [0.0 .. 1.0]
            (pulse_us - MID_US) as f32 / (MAX_US - MID_US) as f32
        }
    }

    pub async fn get(&mut self) -> Speed {
        self.0.wait_for_rising_edge().await;
        let start = Instant::now();
        self.0.wait_for_falling_edge().await;
        let duration = Instant::now().duration_since(start);

        let pulse_us = duration.as_micros() as i32;
        defmt::info!("pulse_us: {:?}", pulse_us);

        Speed::from_percent(Self::map_throttle(pulse_us))
    }
}
