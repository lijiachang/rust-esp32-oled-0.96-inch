use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use embedded_svc::http::client::Client;
use embedded_svc::utils::io::try_read_full;
use esp_idf_svc::http::client::EspHttpConnection;
use log::info;
use chrono::{DateTime, FixedOffset};
use esp_idf_svc::http::client::Configuration as HttpConfig;
// oled display
use embedded_graphics::text::Baseline;
use ssd1306::mode::DisplayConfig;
use ssd1306::rotation::DisplayRotation;
use ssd1306::size::DisplaySize128x64;
use ssd1306::{I2CDisplayInterface, Ssd1306};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_6X13, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::prelude::*;
use serde::Deserialize;

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

/// "Sat, 10 Aug 2024 03:14:05 GMT" -> 2024-08-10 11:14:05
fn parse_and_format_time(date_str: &str) -> Result<String, chrono::ParseError> {
    // 解析时间字符串
    let utc_time = DateTime::parse_from_rfc2822(date_str)?;
    // 创建 UTC+8 时区
    let secs = 8 * 3600;
    let china_timezone = FixedOffset::east_opt(secs).expect("FixedOffset::east out of bounds");
    // 将时间转换为 UTC+8
    let china_time = utc_time.with_timezone(&china_timezone);
    // 格式化输出
    Ok(china_time.format("%Y-%m-%d %H:%M:%S").to_string())
}


#[derive(Deserialize)]
struct TickerPrice {
    symbol: String,
    price: String,
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Hello, world!");

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs)).unwrap(),
        sysloop,
    ).unwrap();

    info!("配置WiFi");
    wifi.set_configuration(
        &Configuration::Client(ClientConfiguration {
            ssid: SSID.try_into().unwrap(),
            bssid: None,
            auth_method: Default::default(),
            password: PASSWORD.try_into().unwrap(),
            channel: None,
            scan_method: Default::default(),
            pmf_cfg: Default::default(),
        },
        )
    ).expect("set_configuration: panic");

    info!("启动WiFi");
    wifi.start().unwrap();

    info!("连接WiFi");
    wifi.connect().unwrap();

    info!("等待底层网络接口启动");
    wifi.wait_netif_up().unwrap();

    info!(
        "获取到IP地址为:{:?}",
        wifi.wifi().sta_netif().get_ip_info()
    );

    // Create HTTPS Connection Handle
    let httpconnection = EspHttpConnection::new(&HttpConfig {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    }).unwrap();

    let mut client = Client::wrap(httpconnection);
    let url = "https://api.binance.com/api/v3/ticker/price?symbols=[%22BTCUSDT%22,%22ETHUSDT%22,%22SOLUSDT%22]";

    // oled
    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio5;
    let scl = peripherals.pins.gpio4;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c: I2cDriver<'static> = I2cDriver::new(i2c, sda, scl, &config).expect("i2c error:");

    let interface = I2CDisplayInterface::new(i2c);

    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    let _ = display.init();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    let date_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X13)
        .text_color(BinaryColor::On)
        .build();

    loop {
        // GET
        let mut resp = client.get(url).unwrap().submit().unwrap();
        info!("响应状态：{}", resp.status());

        let (_headers, mut body) = resp.split();
        let mut buf = [0_u8; 2048]; // 增加缓冲区大小以适应更多数据
        let br = try_read_full(&mut body, &mut buf).unwrap();
        let body = std::str::from_utf8(&buf[0..br]).unwrap();
        info!("响应内容：{body}");

        // Parse JSON
        match serde_json::from_str::<Vec<TickerPrice>>(body) {
            Ok(tickers) => {
                display.clear_buffer();

                // 提取时间展示
                let date = resp.header("date");
                Text::with_baseline(
                    &parse_and_format_time(date.unwrap()).unwrap(),
                    Point::new(0, 0),
                    date_style,
                    Baseline::Top,
                )
                    .draw(&mut display)
                    .unwrap();

                let mut y_offset = 18;

                for (index, ticker) in tickers.iter().enumerate() {
                    let symbol = ticker.symbol.replace("USDT", "");
                    if let Ok(price_float) = ticker.price.parse::<f64>() {
                        let formatted_price = format!("{} {:.2}", symbol, price_float);

                        Text::with_baseline(
                            &formatted_price,
                            Point::new(10, y_offset),
                            text_style,
                            Baseline::Top,
                        )
                            .draw(&mut display)
                            .unwrap();

                        y_offset += 16; // 增加y偏移以显示下一行

                        info!("Displayed: {}", formatted_price);
                    } else {
                        info!("Failed to parse price for {}", symbol);
                    }

                    if index == 3 {
                        break;
                    }
                }

                let _ = display.flush();
            }
            Err(e) => {
                info!("Failed to parse JSON: {}", e);
            }
        }

        FreeRtos::delay_ms(1000); // sleep 1s
    }
}