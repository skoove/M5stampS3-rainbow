use anyhow::Error;
use anyhow::Result;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::http::server::Configuration;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi;
use esp_idf_svc::wifi::BlockingWifi;
use esp_idf_svc::wifi::EspWifi;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use ws2812_esp32_rmt_driver::Ws2812Esp32RmtDriver;

const SSID: &str = env!("SSID");
const PASS: &str = env!("PASS");

struct State {
    led_color: [u8; 3],
    led_on: bool,
    colorloop: bool,
    colorloop_progress: u16,
    last_animation_update: Instant,
}

struct PeripheralsManager {
    led: Ws2812Esp32RmtDriver<'static>,
}

impl State {
    fn new() -> Self {
        Self {
            led_color: [000, 255, 000],
            led_on: true,
            colorloop: false,
            colorloop_progress: 0,
            last_animation_update: Instant::now(),
        }
    }

    fn toggle_led(&mut self) {
        self.led_on = !self.led_on;
    }

    fn set_color(&mut self, color: [u8; 3]) {
        self.led_color = color;
    }

    fn toggle_colorloop(&mut self) {
        self.colorloop = !self.colorloop;
    }
}

impl PeripheralsManager {
    fn set_led_color(&mut self, color: [u8; 3]) -> Result<()> {
        self.led.write_blocking(to_grb(color).into_iter())?;
        Ok(())
    }

    fn turn_off_led(&mut self) -> Result<()> {
        self.set_led_color([000, 000, 000])?;
        Ok(())
    }

    fn update_peripherals(&mut self, state: &mut State) -> Result<()> {
        if state.led_on {
            if state.colorloop {
                // advance colour loop only if its been more than 15 ms since the last update
                if Instant::now().duration_since(state.last_animation_update)
                    > Duration::from_millis(15)
                {
                    let color = hsv::hsv_to_rgb(state.colorloop_progress as f64, 1.0, 1.0);
                    state.last_animation_update = Instant::now();
                    state.colorloop_progress += 1;
                    if state.colorloop_progress >= 360 {
                        state.colorloop_progress = 0;
                    }
                    self.set_led_color(color.into())?;
                }
            } else {
                self.set_led_color(state.led_color)?;
            }
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

    let mut peripherals_manager = PeripheralsManager {
        led: Ws2812Esp32RmtDriver::new(peripherals.rmt.channel0, peripherals.pins.gpio21)?,
    };

    let state = Arc::new(Mutex::new(State::new()));

    let mut server = EspHttpServer::new(&Configuration::default())?;

    server.fn_handler("/index.html", Method::Get, |request| {
        request.into_ok_response()?.write_all(
            b"<html>
                <body>
                    <h1>rainbow m5stamps3, with rust!</h1>
                    <hr>
                    <div>
                        <button onclick='fetch(\"/toggle\")'>toggle led</button>
                        <button onclick='fetch(\"/colorloop\")'>toggle colour loop</button>
                        <input type=\"color\" value='#ff0000' id='color_picker' oninput='set_color()'>
                    </div>
                    <script>
                        function set_color() {
                            const color = document.getElementById('color_picker').value;
                            const r = parseInt(color.substr(1,2), 16);
                            const g = parseInt(color.substr(3,2), 16);
                            const b = parseInt(color.substr(5,2), 16);
                            fetch(`/color?r=${r}&g=${g}&b=${b}`);
                        }
                    </script>
                </body>
            </html>",
        )
    })?;

    // handling toggle button
    let state_ref = state.clone();
    server.fn_handler("/toggle", Method::Get, move |request| {
        log::info!("toggle button was clicked");
        state_ref.lock().unwrap().toggle_led();
        request.into_ok_response()?;
        Ok::<(), Error>(())
    })?;

    // handling colourloop button
    let state_ref = state.clone();
    server.fn_handler("/colorloop", Method::Get, move |request| {
        log::info!("colorloop button was clicked");
        state_ref.lock().unwrap().toggle_colorloop();
        request.into_ok_response()?;
        Ok::<(), Error>(())
    })?;

    // handle set color
    let state_ref = state.clone();
    server.fn_handler("/color", Method::Get, move |request| {
        log::info!("colour change requested");
        log::info!("{}", &request.uri());
        let params: HashMap<_, _>;
        if let Some(query) = &request.uri().split('?').nth(1) {
            params = query
                .split('&')
                .filter_map(|param| {
                    let mut parts = param.split('=');
                    Some((parts.next()?, parts.next()?))
                })
                .collect();
        } else {
            params = HashMap::new();
        }

        if let (Some(r), Some(g), Some(b)) = (params.get("r"), params.get("g"), params.get("b")) {
            state_ref
                .lock()
                .unwrap()
                .set_color([r.parse()?, g.parse()?, b.parse()?]);
        }
        request.into_ok_response()?;
        Ok::<(), Error>(())
    })?;

    loop {
        peripherals_manager
            .update_peripherals(&mut state.lock().unwrap())
            .unwrap();
        // this really probably does not much but it helps with efficency a little
        // ( it does not matter i can gaurantee this code is so bad that the amount of
        // power actually saveed is tiny compared to what could be being saved)
        thread::sleep(Duration::from_millis(1));
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

fn to_grb(rgb: [u8; 3]) -> [u8; 3] {
    [rgb[1], rgb[0], rgb[2]]
}
