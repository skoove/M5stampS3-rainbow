use anyhow::Result;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::Gpio21;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::rmt::RMT;
use esp_idf_svc::http::server::Configuration;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi;
use esp_idf_svc::wifi::BlockingWifi;
use esp_idf_svc::wifi::EspWifi;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use ws2812_esp32_rmt_driver::Ws2812Esp32RmtDriver;

const SSID: &str = env!("SSID");
const PASS: &str = env!("PASS");

struct State {
    led_color: [u8; 3],
    led_on: bool,
}

struct PeripheralsManager {
    led: Ws2812Esp32RmtDriver<'static>,
}

impl State {
    fn new() -> Self {
        Self {
            led_color: [255, 255, 255],
            led_on: true,
        }
    }

    fn toggle_led(&mut self) {
        self.led_on = !self.led_on;
    }
}

impl PeripheralsManager {
    fn new(pin21: Gpio21, rmt: RMT) -> Result<Self> {
        Ok(Self {
            led: Ws2812Esp32RmtDriver::new(rmt.channel0, pin21)?,
        })
    }

    fn set_led_color(&mut self, color: [u8; 3]) -> Result<()> {
        self.led.write_blocking(color.into_iter())?;
        Ok(())
    }

    fn turn_off_led(&mut self) -> Result<()> {
        self.set_led_color([000, 000, 000])?;
        Ok(())
    }

    fn update_peripherals(&mut self, state: &State) -> Result<()> {
        if state.led_on {
            self.set_led_color(state.led_color)?;
        } else {
            self.turn_off_led()?;
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // wifi
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    connect_wifi(&mut wifi)?;

    let mut peripherals_manager =
        PeripheralsManager::new(peripherals.pins.gpio21, peripherals.rmt)?;
    let state = Arc::new(Mutex::new(State::new()));

    let mut server = EspHttpServer::new(&Configuration::default())?;

    server.fn_handler("/index.html", Method::Get, |request| {
        request.into_ok_response()?.write_all(
            b"<html>
                <body>
                    <h1>Hello, World!</h1>
                        <button onclick='fetch(\"/toggle\")'>toggle led</button>
                </body>
            </html>",
        )
    })?;

    // handling toggle button
    let state_ref = state.clone();
    server.fn_handler("/toggle", Method::Get, move |request| {
        log::info!("toggle button was clicked");
        state_ref.lock().unwrap().toggle_led();
        request.into_ok_response().unwrap().write_all(b"toggled")
    })?;

    loop {
        peripherals_manager
            .update_peripherals(&state.lock().unwrap())
            .unwrap();
        thread::sleep(Duration::from_millis(150));
    }
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> Result<()> {
    let wifi_configuration = wifi::Configuration::Client(wifi::ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        bssid: None,
        auth_method: wifi::AuthMethod::WPA2Personal,
        password: PASS.try_into().unwrap(),
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    log::info!("wifi started!");

    wifi.connect()?;
    log::info!("wifi connected!");
    log::info!(
        "attempting to connect to wifi..
        ssid: {SSID}
        pass: {PASS}"
    );

    wifi.wait_netif_up()?;
    log::info!("netif up!");
    Ok(())
}
