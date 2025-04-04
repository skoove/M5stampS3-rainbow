use esp_idf_svc::hal::prelude::*;
use hsv::hsv_to_rgb;
use ws2812_esp32_rmt_driver::Ws2812Esp32RmtDriver;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let led_pin = peripherals.pins.gpio21;
    let channel = peripherals.rmt.channel0;
    let mut driver = Ws2812Esp32RmtDriver::new(channel, led_pin).unwrap();
    // let colors: [[u8; 3]; 4] = [
    //     [255, 255, 255],
    //     [255, 000, 000],
    //     [000, 255, 000],
    //     [000, 000, 255],
    // ];

    loop {
        for hue in 0..=360 {
            let led_data: [u8; 3] = hsv_to_rgb(hue as f64, 1.0, 1.0).into();
            log::info!("{:#?}", led_data);
            driver.write_blocking(led_data.into_iter()).unwrap();
        }
    }
}
