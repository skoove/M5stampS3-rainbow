use esp_idf_svc::hal::prelude::*;
use ws2812_esp32_rmt_driver::Ws2812Esp32RmtDriver;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let led_pin = peripherals.pins.gpio21;
    let channel = peripherals.rmt.channel0;
    let mut driver = Ws2812Esp32RmtDriver::new(channel, led_pin).unwrap();
    let colors: [[u8; 3]; 4] = [
        [255, 255, 255],
        [255, 000, 000],
        [000, 255, 000],
        [000, 000, 255],
    ];

    loop {
        for color in &colors {
            let led_data: [u8; 3] = *color;
            std::thread::sleep(std::time::Duration::from_millis(500));
            driver.write_blocking(led_data.into_iter()).unwrap();
        }
    }
}
