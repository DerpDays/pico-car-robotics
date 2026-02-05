use embassy_rp::{
    Peri,
    peripherals::{PIN_4, PIN_5, PIN_6, PIN_7, PWM_SLICE2, PWM_SLICE3},
    pwm::{Config, Pwm, PwmOutput, SetDutyCycle},
};

pub struct Motors {
    left: PwmOutputAb<'static>,
    right: PwmOutputAb<'static>,
}
pub struct PwmOutputAb<'a> {
    pub a: PwmOutput<'a>,
    pub b: PwmOutput<'a>,
}
impl<'a> TryFrom<Pwm<'a>> for PwmOutputAb<'a> {
    type Error = ();

    fn try_from(value: Pwm<'a>) -> Result<Self, Self::Error> {
        let (a, b) = value.split();
        Ok(Self {
            a: a.ok_or(())?,
            b: b.ok_or(())?,
        })
    }
}

type LeftMotor = (
    Peri<'static, PWM_SLICE2>,
    Peri<'static, PIN_4>,
    Peri<'static, PIN_5>,
);
type RightMotor = (
    Peri<'static, PWM_SLICE3>,
    Peri<'static, PIN_6>,
    Peri<'static, PIN_7>,
);

/// Speed is a scalar value where:
/// 1 => max speed
/// 0 => motor off
/// -1 => max reverse speed
#[derive(Copy, Clone)]
pub struct Speed(f32);
impl Speed {
    const OFF: Self = Speed(0.);
    const MAX_FORWARD: Self = Speed(1.);
    const MAX_REVERSE: Self = Speed(-1.);
    //
    pub fn from_percent(val: f32) -> Self {
        Self(val.clamp(-1., 1.))
    }
}

impl Motors {
    pub fn init(left: LeftMotor, right: RightMotor) -> Self {
        let desired_freq_hz = 25_000;
        let clock_freq_hz = embassy_rp::clocks::clk_sys_freq();
        let divider = 16u8;
        let period = (clock_freq_hz / (desired_freq_hz * divider as u32)) as u16 - 1;

        let mut c = Config::default();
        c.top = period;
        c.divider = divider.into();

        let left_motor = Pwm::new_output_ab(left.0, left.1, left.2, c.clone());
        let right_motor = Pwm::new_output_ab(right.0, right.1, right.2, c);

        Self {
            left: PwmOutputAb::try_from(left_motor).unwrap(),
            right: PwmOutputAb::try_from(right_motor).unwrap(),
        }
    }

    /// Drive each of the motors at the given speed.
    pub fn drive_speed(&mut self, left: Speed, right: Speed) {
        match left.0 {
            // forward
            f32::MIN_POSITIVE..=1.0f32 => {
                let _ = self.left.a.set_duty_cycle_percent((left.0 * 100.) as u8);
                let _ = self.left.b.set_duty_cycle_fully_off();
            }
            // reverse
            -1.0f32..0.0f32 => {
                let _ = self.left.a.set_duty_cycle_fully_off();
                let _ = self.left.b.set_duty_cycle_percent((-left.0 * 100.) as u8);
            }
            // turn motor off
            0f32 => {
                let _ = self.left.a.set_duty_cycle_fully_off();
                let _ = self.left.b.set_duty_cycle_fully_off();
            }
            _ => {
                unreachable!("speed provided must be between -1=..=1");
            }
        }

        match right.0 {
            // forward
            f32::MIN_POSITIVE..=1.0f32 => {
                let _ = self.right.a.set_duty_cycle_percent((right.0 * 100.) as u8);
                let _ = self.right.b.set_duty_cycle_fully_off();
            }
            // reverse
            -1.0f32..0.0f32 => {
                let _ = self.right.a.set_duty_cycle_fully_off();
                let _ = self.right.b.set_duty_cycle_percent((-right.0 * 100.) as u8);
            }
            // turn motor off
            0f32 => {
                let _ = self.right.a.set_duty_cycle_fully_off();
                let _ = self.right.b.set_duty_cycle_fully_off();
            }
            _ => {
                unreachable!("speed provided must be between -1=..=1");
            }
        }
    }
}
