use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use embedded_svc::http::client::Client;
use embedded_svc::utils::io::try_read_full;
use esp_idf_svc::http::client::EspHttpConnection;
use log::{info, error, warn};
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
use std::time::Duration;

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");
const REQUEST_TIMEOUT: u64 = 10; // 10 seconds timeout

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

    let peripherals = match Peripherals::take() {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to take peripherals: {:?}", e);
            return;
        }
    };

    let sysloop = match EspSystemEventLoop::take() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to take system event loop: {:?}", e);
            return;
        }
    };

    let nvs = match EspDefaultNvsPartition::take() {
        Ok(n) => n,
        Err(e) => {
            error!("Failed to take NVS partition: {:?}", e);
            return;
        }
    };

    let wifi_driver = match EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs)) {
        Ok(w) => w,
        Err(e) => {
            error!("Failed to create WiFi driver: {:?}", e);
            return;
        }
    };

    let mut wifi = match BlockingWifi::wrap(wifi_driver, sysloop) {
        Ok(w) => w,
        Err(e) => {
            error!("Failed to create blocking WiFi: {:?}", e);
            return;
        }
    };

    info!("配置WiFi");
    if let Err(e) = wifi.set_configuration(
        &Configuration::Client(ClientConfiguration {
            ssid: SSID.try_into().unwrap(),
            bssid: None,
            auth_method: Default::default(),
            password: PASSWORD.try_into().unwrap(),
            channel: None,
            scan_method: Default::default(),
            pmf_cfg: Default::default(),
        })
    ) {
        error!("Failed to configure WiFi: {:?}", e);
        return;
    }

    info!("启动WiFi");
    if let Err(e) = wifi.start() {
        error!("Failed to start WiFi: {:?}", e);
        return;
    }

    info!("连接WiFi");
    if let Err(e) = wifi.connect() {
        error!("Failed to connect to WiFi: {:?}", e);
        return;
    }

    info!("等待底层网络接口启动");
    if let Err(e) = wifi.wait_netif_up() {
        error!("Network interface failed to come up: {:?}", e);
        return;
    }

    info!(
        "获取到IP地址为:{:?}",
        wifi.wifi().sta_netif().get_ip_info()
    );

    // Create HTTPS Connection Handle with timeout
    let httpconnection = match EspHttpConnection::new(&HttpConfig {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        timeout: Some(Duration::from_secs(REQUEST_TIMEOUT)),
        ..Default::default()
    }) {
        Ok(conn) => conn,
        Err(e) => {
            error!("Failed to create HTTP connection: {:?}", e);
            return;
        }
    };

    let mut client = Client::wrap(httpconnection);
    let url = "https://api.binance.com/api/v3/ticker/price?symbols=[%22BTCUSDT%22,%22ETHUSDT%22,%22SOLUSDT%22]";

    // oled setup
    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio5;
    let scl = peripherals.pins.gpio4;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c = match I2cDriver::new(i2c, sda, scl, &config) {
        Ok(i2c) => i2c,
        Err(e) => {
            error!("I2C error: {:?}", e);
            return;
        }
    };

    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    if let Err(e) = display.init() {
        error!("Failed to initialize display: {:?}", e);
        return;
    }

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    let date_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X13)
        .text_color(BinaryColor::On)
        .build();

    loop {
        // GET with error handling
        let mut resp = match client.get(url) {
            Ok(req) => match req.submit() {
                Ok(resp) => {
                    info!("响应状态：{}", resp.status());
                    resp
                },
                Err(e) => {
                    error!("Failed to submit request: {:?}", e);
                    // Clear display and show error
                    display.clear_buffer();
                    Text::with_baseline(
                        "Network error",
                        Point::new(0, 0),
                        date_style,
                        Baseline::Top,
                    )
                        .draw(&mut display)
                        .ok();
                    display.flush().ok();
                    FreeRtos::delay_ms(5000); // Wait 5 seconds before retrying
                    continue;
                }
            },
            Err(e) => {
                error!("Failed to create request: {:?}", e);
                FreeRtos::delay_ms(5000);
                continue;
            }
        };

        let (_headers, mut body) = resp.split();
        let mut buf = [0_u8; 2048];

        let br = match try_read_full(&mut body, &mut buf) {
            Ok(br) => br,
            Err(e) => {
                error!("Failed to read response body: {:?}", e);
                FreeRtos::delay_ms(1000);
                continue;
            }
        };

        let body = match std::str::from_utf8(&buf[0..br]) {
            Ok(b) => b,
            Err(e) => {
                error!("Invalid UTF-8 in response: {:?}", e);
                FreeRtos::delay_ms(1000);
                continue;
            }
        };

        info!("响应内容：{body}");

        // Clear display for new data
        display.clear_buffer();

        // Parse JSON and update display
        match serde_json::from_str::<Vec<TickerPrice>>(body) {
            Ok(tickers) => {
                // Extract and display time
                if let Some(date_str) = resp.header("date") {
                    match parse_and_format_time(date_str) {
                        Ok(formatted_time) => {
                            Text::with_baseline(
                                &formatted_time,
                                Point::new(0, 0),
                                date_style,
                                Baseline::Top,
                            )
                                .draw(&mut display)
                                .ok();
                        },
                        Err(e) => {
                            warn!("Failed to parse date: {:?}", e);
                            // Still show something as date
                            Text::with_baseline(
                                "Time error",
                                Point::new(0, 0),
                                date_style,
                                Baseline::Top,
                            )
                                .draw(&mut display)
                                .ok();
                        }
                    }
                }

                let mut y_offset = 18;

                for (index, ticker) in tickers.iter().enumerate() {
                    let symbol = ticker.symbol.replace("USDT", "");
                    match ticker.price.parse::<f64>() {
                        Ok(price_float) => {
                            let formatted_price = format!("{} {:.2}", symbol, price_float);

                            Text::with_baseline(
                                &formatted_price,
                                Point::new(10, y_offset),
                                text_style,
                                Baseline::Top,
                            )
                                .draw(&mut display)
                                .ok();

                            y_offset += 16;
                            info!("Displayed: {}", formatted_price);
                        },
                        Err(e) => {
                            warn!("Failed to parse price for {}: {:?}", symbol, e);
                        }
                    }

                    if index == 3 {
                        break;
                    }
                }
            },
            Err(e) => {
                error!("Failed to parse JSON: {}", e);
                // Display error on OLED
                Text::with_baseline(
                    "JSON Error",
                    Point::new(10, 20),
                    text_style,
                    Baseline::Top,
                )
                    .draw(&mut display)
                    .ok();
            }
        }

        // Flush display changes but handle errors gracefully
        if let Err(e) = display.flush() {
            error!("Display flush error: {:?}", e);
        }

        FreeRtos::delay_ms(1000); // sleep 1s
    }
}