#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{adc, gpio, pwm, uart};
use embassy_time::Duration;

use heapless::Vec;

use {defmt_rtt as _, panic_probe as _};
use rpi_pico_entry::debouncer::Debouncer;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut uart_0 = uart::Uart::new_blocking(p.UART0, p.PIN_16, p.PIN_17, uart::Config::default());
    uart_0
        .blocking_write("Hello World!\r\n".as_bytes())
        .unwrap();
    info!("board ok");

    let mut pwm_0_config: pwm::Config = Default::default();
    pwm_0_config.phase_correct = true;

    let divider = 0x20u8;

    let mut top = 125000000f32; // frequency of rp2040
    top *= 20.0;
    top /= 1000.0; // number of cycles for 20ms
    top /= 2.0; // phase correct wave
    top /= divider as f32;
    top -= 1.0;
    let top = top as u16;
    info!("pwm cycle: {}", top);
    
    let ts_vec: [f32; 4] = [1.5, 1.2, 1.5, 1.8];
    let width_vec = ts_vec.into_iter()
        .map(|ts| top as f32 * ts / 20.0)
        .map(|compare| compare as u16)
        .collect::<Vec<u16, 8>>();
    let mut ptr = usize::MIN;
    info!("div: {:?}", width_vec.as_slice());
    
    pwm_0_config.divider = divider.into();
    pwm_0_config.top = top;
    pwm_0_config.compare_a = width_vec[ptr]; // range: 0.5ms-2.5ms, 8-40, medium: 1.5ms, 13.33
    let mut pwm_0 = pwm::Pwm::new_output_a(p.PWM_SLICE0,  p.PIN_0, pwm_0_config.clone());

    let mut button = Debouncer::new(
        gpio::Input::new(p.PIN_1, gpio::Pull::Up),
        Duration::from_millis(20),
    );

    while let level = button.debounce().await {
        if level == gpio::Level::Low { continue; }

        ptr += 1;
        ptr %= width_vec.len();
        pwm_0_config.compare_a = width_vec[ptr];
        pwm_0.set_config(&pwm_0_config);
    }
}
