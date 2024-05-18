use embassy_rp::gpio;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::pubsub::Subscriber;
use embassy_time::{Duration, Timer};

pub struct Config {
    pub pin: gpio::AnyPin,
    pub duration: Duration,
}

pub struct Channel {
    pub enabled: Subscriber<'static, ThreadModeRawMutex, bool, 4, 4, 4>,
}

#[embassy_executor::task]
pub async fn run(config: Config, mut channel: Channel) {
    let mut led_onboard = gpio::Output::new(config.pin, gpio::Level::Low);
    for _ in 0..3 {
        Timer::after(config.duration).await;
        led_onboard.set_high();
        Timer::after(config.duration).await;
        led_onboard.set_low();
    }

    loop {
        channel.enabled.next_message_pure().await;
        led_onboard.set_high();
        Timer::after(config.duration).await;
        led_onboard.set_low();
        Timer::after(config.duration).await;
    }
}
