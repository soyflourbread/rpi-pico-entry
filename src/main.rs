#![no_std]
#![no_main]

mod debouncer;
mod led_ctrl;
mod toggle;

use core::sync::atomic::AtomicBool;

use embassy_executor::Spawner;
use embassy_rp::{gpio, pwm, uart, Peripheral};
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::pubsub::{PubSubBehavior, PubSubChannel, Publisher, Subscriber};
use embassy_time::{Duration, Ticker, Timer};

use defmt::*;
use embassy_rp::uart::{Instance, RxPin, TxPin};
use {defmt_rtt as _, panic_probe as _};

const BRIGHTNESS_THRESHOLD: u16 = 6000;

static ENABLED: AtomicBool = AtomicBool::new(false);

static CHANNEL_ENABLED: PubSubChannel<ThreadModeRawMutex, bool, 4, 4, 4> = PubSubChannel::new();
static CHANNEL_LED: Channel<ThreadModeRawMutex, (usize, u16), 64> = Channel::new();

fn led_ctrl_run(spawner: &Spawner) {
    let tick = Duration::from_millis(10);
    let config_0 = led_ctrl::Config {
        id: 0,
        k: 8,
        multiplier: 1.07,
        tick,
    };
    let config_1 = led_ctrl::Config {
        id: 1,
        k: 8,
        multiplier: 1.1,
        tick,
    };
    let to_channel = || led_ctrl::Channel {
        running: CHANNEL_ENABLED.subscriber().unwrap(),
        led: CHANNEL_LED.sender(),
    };
    unwrap!(spawner.spawn(led_ctrl::run(config_0, to_channel())));
    unwrap!(spawner.spawn(led_ctrl::run(config_1, to_channel())));
}

fn toggle_run(spawner: &Spawner, pin: gpio::AnyPin) {
    let duration = Duration::from_millis(20);
    let config = toggle::Config { pin, duration };
    let state = toggle::State { enabled: &ENABLED };
    let channel = toggle::Channel {
        enabled: CHANNEL_ENABLED.publisher().unwrap(),
    };
    unwrap!(spawner.spawn(toggle::run(config, state, channel)));
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut uart_0 = uart::Uart::new_blocking(p.UART0, p.PIN_16, p.PIN_17, uart::Config::default());
    uart_0
        .blocking_write("Hello World!\r\n".as_bytes())
        .unwrap();

    let mut led_onboard = gpio::Output::new(p.PIN_25, gpio::Level::Low);
    for _ in 0..3 {
        Timer::after_millis(100).await;
        led_onboard.set_high();
        Timer::after_millis(100).await;
        led_onboard.set_low();
    }

    info!("board ok, starting");

    let mut pwm_0_config: pwm::Config = Default::default();
    pwm_0_config.top = 0x8000;
    pwm_0_config.compare_a = 8;
    pwm_0_config.compare_b = 8;
    let mut pwm_0 = pwm::Pwm::new_output_ab(p.PWM_SLICE0, p.PIN_0, p.PIN_1, pwm_0_config.clone());

    led_ctrl_run(&spawner);
    toggle_run(&spawner, gpio::AnyPin::from(p.PIN_2));

    loop {
        let (led_id, brightness) = CHANNEL_LED.receive().await;
        match led_id {
            0 => pwm_0_config.compare_a = brightness,
            1 => pwm_0_config.compare_b = brightness,
            _ => {}
        }
        pwm_0.set_config(&pwm_0_config);
    }
}
