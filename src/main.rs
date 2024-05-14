#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::{gpio, Peripheral, pwm, uart};
use embassy_time::{Duration, Ticker, Timer};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Sender};

use defmt::*;
use embassy_rp::uart::{Instance, RxPin, TxPin};
use {defmt_rtt as _, panic_probe as _};

const BRIGHTNESS_THRESHOLD: u16 = 4000;

#[embassy_executor::task]
async fn led_0_prop(control: Sender<'static, ThreadModeRawMutex, (usize, u16), 64>) {
    let mut ticker = Ticker::every(Duration::from_millis(10));
    loop {
        let mut brightness = 8f32;
        while (brightness as u16) < BRIGHTNESS_THRESHOLD {
            brightness *= 1.05;
            control.send((0, brightness as u16)).await;
            ticker.next().await;
        }
    }
}

#[embassy_executor::task]
async fn led_1_prop(control: Sender<'static, ThreadModeRawMutex, (usize, u16), 64>) {
    let mut ticker = Ticker::every(Duration::from_millis(10));
    loop {
        let mut brightness = 8f32;
        while (brightness as u16) < BRIGHTNESS_THRESHOLD {
            brightness *= 1.1;
            control.send((1, brightness as u16)).await;
            ticker.next().await;
        }
    }
}

static CHANNEL: Channel<ThreadModeRawMutex, (usize, u16), 64> = Channel::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut uart_0 = uart::Uart::new_blocking(
        p.UART0, p.PIN_16, p.PIN_17,
        uart::Config::default()
    );
    uart_0.blocking_write("Hello World!\r\n".as_bytes()).unwrap();

    let mut led_onboard = gpio::Output::new(p.PIN_25, gpio::Level::Low);
    for _ in 0..3 {
        Timer::after_millis(100).await;
        led_onboard.set_high();
        Timer::after_millis(100).await;
        led_onboard.set_low();
    }

    info!("board ok");

    let mut pwm_0_config: pwm::Config = Default::default();
    pwm_0_config.top = 0x8000;
    pwm_0_config.compare_a = 8;
    pwm_0_config.compare_b = 8;
    let mut pwm_0 = pwm::Pwm::new_output_ab(p.PWM_SLICE0, p.PIN_0, p.PIN_1, pwm_0_config.clone());

    unwrap!(spawner.spawn(led_0_prop(CHANNEL.sender())));
    unwrap!(spawner.spawn(led_1_prop(CHANNEL.sender())));

    loop {
        let (led_id, brightness) = CHANNEL.receive().await;
        match led_id {
            0 => pwm_0_config.compare_a = brightness,
            1 => pwm_0_config.compare_b = brightness,
            _ => {}
        }
        pwm_0.set_config(&pwm_0_config);
    }
}
