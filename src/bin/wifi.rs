use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use log::info;
use std::env;


const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

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

}