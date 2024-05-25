#![no_std]
#![no_main]

use core::sync::atomic::AtomicBool;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{gpio, pwm, uart, adc};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::pubsub::PubSubChannel;
use embassy_time::Duration;

use {defmt_rtt as _, panic_probe as _};
use rpi_pico_entry::{status, toggle};

static ENABLED: AtomicBool = AtomicBool::new(false);

static CHANNEL_ENABLED: PubSubChannel<ThreadModeRawMutex, bool, 4, 4, 4> = PubSubChannel::new();

fn status_run(spawner: &Spawner, pin: gpio::AnyPin) {
    let duration = Duration::from_millis(100);
    let config = status::Config { pin, duration };
    let channel = status::Channel {
        enabled: CHANNEL_ENABLED.subscriber().unwrap(),
    };
    unwrap!(spawner.spawn(status::run(config, channel)));
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
    info!("board ok");

    let mut pwm_0_config: pwm::Config = Default::default();
    pwm_0_config.phase_correct = true;
    pwm_0_config.divider = 0xFF.into();
    pwm_0_config.top = 0x80;
    pwm_0_config.compare_a = 0x0;
    let mut pwm_0 = pwm::Pwm::new_output_a(p.PWM_SLICE0, p.PIN_0, pwm_0_config.clone());

    status_run(&spawner, gpio::AnyPin::from(p.PIN_25));
    toggle_run(&spawner, gpio::AnyPin::from(p.PIN_2));

    let mut chan_enabled_sub = CHANNEL_ENABLED.subscriber().unwrap();
    while let enabled = chan_enabled_sub.next_message_pure().await {
        info!("enabled: {}", enabled);
        pwm_0_config.compare_a = if enabled { 0x10 } else { 0x0 };
        pwm_0.set_config(&pwm_0_config);
    }
}
