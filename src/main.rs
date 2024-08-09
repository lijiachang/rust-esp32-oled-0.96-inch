use embedded_graphics::Drawable;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::text::{Baseline, Text};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::prelude::*;
use ssd1306::{I2CDisplayInterface, rotation::DisplayRotation, size::DisplaySize128x64, Ssd1306};
use ssd1306::mode::{BufferedGraphicsMode, DisplayConfig};
use ssd1306::prelude::I2CInterface;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let peripherals = Peripherals::take().unwrap();
    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio5;
    let scl = peripherals.pins.gpio4;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c: I2cDriver<'static> = I2cDriver::new(i2c, sda, scl, &config).expect("i2c new error: ");

    let interface = I2CDisplayInterface::new(i2c);

    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0).into_buffered_graphics_mode();
    display.init().unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    let mut num = 0;

    loop {
        display.clear_buffer();

        Text::with_baseline(
            "num",
            Point::new(10, 0),
            text_style,
            Baseline::Top,
        )
            .draw(&mut display)
            .unwrap();

        Text::with_baseline(
            format!("{}", num).as_str(),
            Point::new(80, 0),
            text_style,
            Baseline::Top,
        )
            .draw(&mut display)
            .unwrap();


        Text::with_baseline(
            "Hello Wold!",
            Point::new(10, 20),
            text_style,
            Baseline::Top,
        )
            .draw(&mut display)
            .unwrap();

        Text::with_baseline(
            "Hello Rust!",
            Point::new(10, 40),
            text_style,
            Baseline::Top,
        )
            .draw(&mut display)
            .unwrap();

        display.flush().unwrap();

        FreeRtos::delay_ms(1000); // sleep 1s

        num += 1;
    }
}
