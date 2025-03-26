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
use ssd1306::mode::{BufferedGraphicsMode, DisplayConfig};
use ssd1306::rotation::DisplayRotation;
use ssd1306::size::DisplaySize128x64;
use ssd1306::{I2CDisplayInterface, Ssd1306};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_6X13, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use embedded_graphics::mono_font::MonoTextStyle;
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::prelude::*;
use serde::Deserialize;
// 看门狗
use esp_idf_sys::esp_restart;
use ssd1306::prelude::I2CInterface;
use std::time::Duration;
use esp_idf_svc::systime::EspSystemTime;

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

    // 初始化软件看门狗
    let sys_time = EspSystemTime {};
    let mut last_feed_time = sys_time.now().as_millis(); // 获取毫秒时间戳
    let watchdog_timeout_ms = 60000; // 60秒超时 (60000毫秒)

    info!("初始化软件看门狗，超时时间为60秒");

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

    // 更新看门狗时间
    last_feed_time = sys_time.now().as_millis();

    info!("启动WiFi");
    wifi.start().unwrap();

    info!("连接WiFi");
    wifi.connect().unwrap();

    // 更新看门狗时间
    last_feed_time = sys_time.now().as_millis();

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

    // 连续错误计数器
    let mut consecutive_errors = 0;
    let max_consecutive_errors = 5;

    // 无更新迭代计数器
    let mut iterations_without_update = 0;
    let max_iterations_without_update = 300; // 约5分钟（如果循环每秒运行一次）

    loop {
        // 检查看门狗是否超时
        let now_ms = sys_time.now().as_millis();
        if now_ms - last_feed_time > watchdog_timeout_ms {
            info!("看门狗超时！重启系统...");
            display.clear_buffer();
            Text::with_baseline(
                "看门狗超时，重启中...",
                Point::new(0, 32),
                text_style,
                Baseline::Top,
            )
                .draw(&mut display)
                .unwrap();
            display.flush().unwrap();
            FreeRtos::delay_ms(1000);

            // 重启设备
            unsafe {
                esp_restart();
            }
        }

        // 复位看门狗
        last_feed_time = now_ms;

        // 检查是否太长时间没有成功更新
        if iterations_without_update >= max_iterations_without_update {
            info!("太长时间没有成功更新 ({}次)，准备重启...", iterations_without_update);
            // 显示重启信息
            display.clear_buffer();
            Text::with_baseline(
                "系统即将重启...",
                Point::new(0, 32),
                text_style,
                Baseline::Top,
            )
                .draw(&mut display)
                .unwrap();
            display.flush().unwrap();

            FreeRtos::delay_ms(1000);

            // 重启设备
            unsafe {
                esp_restart();
            }
        }

        info!("获取API数据...");

        // GET请求
        let resp_result = client.get(url);
        if let Err(e) = resp_result {
            info!("HTTP请求创建失败: {:?}", e);
            consecutive_errors += 1;
            iterations_without_update += 1;

            // 显示错误消息
            display_error(&mut display, &format!("HTTP请求错误: {}", consecutive_errors), text_style);

            handle_consecutive_errors(&mut client, &mut consecutive_errors,
                                      max_consecutive_errors, &mut wifi, &mut display, text_style);

            FreeRtos::delay_ms(1000);
            continue;
        }

        let submit_result = resp_result.unwrap().submit();
        if let Err(e) = submit_result {
            info!("HTTP请求提交失败: {:?}", e);
            consecutive_errors += 1;
            iterations_without_update += 1;

            // 显示错误消息
            display_error(&mut display, &format!("网络错误: {}", consecutive_errors), text_style);

            handle_consecutive_errors(&mut client, &mut consecutive_errors,
                                      max_consecutive_errors, &mut wifi, &mut display, text_style);

            FreeRtos::delay_ms(1000);
            continue;
        }

        let mut resp = submit_result.unwrap();
        info!("响应状态：{}", resp.status());

        // 获取日期头
        // let date = resp.header("date");

        let (_headers, mut body) = resp.split();
        let mut buf = [0_u8; 2048];

        // 使用正确的错误处理方式读取响应体
        let read_result = try_read_full(&mut body, &mut buf);
        if let Err(e) = read_result {
            info!("读取响应体失败: {:?}", e);
            consecutive_errors += 1;
            iterations_without_update += 1;

            // 显示错误消息
            display_error(&mut display, &format!("读取响应错误: {}", consecutive_errors), text_style);

            handle_consecutive_errors(&mut client, &mut consecutive_errors,
                                      max_consecutive_errors, &mut wifi, &mut display, text_style);

            FreeRtos::delay_ms(1000);
            continue;
        }

        let br = read_result.unwrap();

        let body_result = std::str::from_utf8(&buf[0..br]);
        if let Err(e) = body_result {
            info!("解析响应体为UTF-8失败: {:?}", e);
            consecutive_errors += 1;
            iterations_without_update += 1;

            // 显示错误消息
            display_error(&mut display, "UTF-8解析错误", text_style);

            FreeRtos::delay_ms(1000);
            continue;
        }

        let body = body_result.unwrap();
        info!("响应内容：{body}");

        // Parse JSON
        match serde_json::from_str::<Vec<TickerPrice>>(body) {
            Ok(tickers) => {
                // 成功：重置错误计数器
                consecutive_errors = 0;
                iterations_without_update = 0;

                display.clear_buffer();

                // 提取时间展示
                let date = resp.header("date");
                if let Some(date_str) = date {
                    if let Ok(formatted_time) = parse_and_format_time(date_str) {
                        Text::with_baseline(
                            &formatted_time,
                            Point::new(0, 0),
                            date_style,
                            Baseline::Top,
                        )
                            .draw(&mut display)
                            .unwrap();
                    }
                }

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
                info!("解析JSON失败: {}", e);
                consecutive_errors += 1;
                iterations_without_update += 1;

                // 显示错误消息
                display_error(&mut display, "JSON解析错误", text_style);
            }
        }

        FreeRtos::delay_ms(1000); // 睡眠1秒
    }
}

// 显示错误消息
fn display_error(display: &mut Ssd1306<I2CInterface<I2cDriver<'static>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>,
                 message: &str,
                 style: MonoTextStyle<BinaryColor>) {
    display.clear_buffer();
    Text::with_baseline(
        message,
        Point::new(0, 32),
        style,
        Baseline::Top,
    )
        .draw(display)
        .unwrap();
    display.flush().unwrap();
}

// 处理连续错误
fn handle_consecutive_errors(
    client: &mut Client<EspHttpConnection>,
    consecutive_errors: &mut u32,
    max_consecutive_errors: u32,
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    display: &mut Ssd1306<I2CInterface<I2cDriver<'static>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>,
    text_style: MonoTextStyle<BinaryColor>
) {
    // 如果连续错误太多，尝试重置连接
    if *consecutive_errors >= max_consecutive_errors {
        info!("连续错误过多 ({})，尝试重置连接...", consecutive_errors);

        // 显示重连信息
        display_error(display, "正在重新连接...", text_style);

        // 重建HTTP连接
        if let Ok(new_conn) = EspHttpConnection::new(&HttpConfig {
            use_global_ca_store: true,
            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
            ..Default::default()
        }) {
            *client = Client::wrap(new_conn);
            info!("HTTP客户端已重置");

            // 检查WiFi连接
            if wifi.is_connected().unwrap_or(false) {
                info!("WiFi仍然连接");
            } else {
                info!("WiFi已断开，尝试重连...");
                // 完整的WiFi重连序列
                let _ = wifi.stop();
                FreeRtos::delay_ms(1000);

                if let Err(e) = wifi.start() {
                    info!("WiFi启动失败: {:?}", e);
                } else if let Err(e) = wifi.connect() {
                    info!("WiFi连接失败: {:?}", e);
                } else {
                    info!("WiFi重连成功");
                    if let Err(e) = wifi.wait_netif_up() {
                        info!("等待网络接口失败: {:?}", e);
                    } else {
                        info!("网络接口已启动");
                    }
                }
            }

            // 重置错误计数器
            *consecutive_errors = 0;
        } else {
            info!("重置HTTP连接失败");
        }
    }
}