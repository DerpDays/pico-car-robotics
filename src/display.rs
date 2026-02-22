use display_interface_i2c::I2CInterface;
use embassy_rp::{
    Peri,
    i2c::{Async, I2c},
    peripherals::{I2C0, PIN_0, PIN_1},
};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::{Point, *},
    primitives::Rectangle,
    text::{Baseline, Text},
};
use oled_async::{Builder, displays::sh1106::Sh1106_128_64, mode::GraphicsMode};

use crate::Irqs;

type DisplayPeripherals = (
    Peri<'static, I2C0>,
    Peri<'static, PIN_1>,
    Peri<'static, PIN_0>,
);

pub struct Display(GraphicsMode<Sh1106_128_64, I2CInterface<I2c<'static, I2C0, Async>>>);

impl core::ops::Deref for Display {
    type Target = GraphicsMode<Sh1106_128_64, I2CInterface<I2c<'static, I2C0, Async>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl core::ops::DerefMut for Display {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display {
    pub async fn new(peripherals: DisplayPeripherals) -> Result<Self, ()> {
        let config = embassy_rp::i2c::Config::default();
        let i2c = embassy_rp::i2c::I2c::new_async(
            peripherals.0,
            peripherals.1,
            peripherals.2,
            Irqs,
            config,
        );
        let di = I2CInterface::new(i2c, 0x3C, 0x40);

        let raw_disp = Builder::new(oled_async::displays::sh1106::Sh1106_128_64 {}).connect(di);

        let mut display: GraphicsMode<_, _> = raw_disp.into();
        display
            .init()
            .await
            .map_err(|e| defmt::error!("error initialising display: {:?}", e))?;

        Ok(Self(display))
    }
}

#[embassy_executor::task]
pub async fn drive_display(mut display: Display) {
    let raw: ImageRaw<BinaryColor> = ImageRaw::new(include_bytes!("./rust.raw"), 64);

    let im = Image::new(&raw, Point::new(32, 0));

    // loop {
    // loop {
    if let Err(e) = display.fill_solid(
        &Rectangle::new(Point::zero(), Size::new(128, 64)),
        BinaryColor::Off,
    ) {
        defmt::error!("failed to clear display {:?}", e);
    };

    if let Err(e) = im.draw(&mut *display) {
        defmt::error!("failed to draw to display {:?}", e);
    };
    if let Err(e) = display.flush().await {
        defmt::error!("error flushing display: {:?}", e);
    };
    //     Timer::after(Duration::from_millis(1000)).await;
    // }
}

// #[inline]
// fn handle_display_err(e: sh1106::Error<embassy_rp::i2c::Error, ()>) {
//     match e {
//         sh1106::Error::Comm(e) => defmt::error!("display: communication error {:?}", e),
//         sh1106::Error::Pin(_) => defmt::error!("display: failed to set pin"),
//     }
// }
