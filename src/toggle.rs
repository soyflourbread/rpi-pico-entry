use core::sync::atomic::AtomicBool;

use embassy_rp::gpio;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::pubsub::Publisher;
use embassy_time::Duration;

use crate::debouncer::Debouncer;

pub struct Config {
    pub pin: gpio::AnyPin,
    pub duration: Duration,
}

pub struct State {
    pub enabled: &'static AtomicBool,
}

pub struct Channel {
    pub enabled: Publisher<'static, ThreadModeRawMutex, bool, 4, 4, 4>,
}

#[embassy_executor::task]
pub async fn run(config: Config, state: State, channel: Channel) {
    let mut button = Debouncer::new(
        gpio::Input::new(config.pin, gpio::Pull::Up),
        config.duration,
    );

    loop {
        let pressed = button.debounce().await == gpio::Level::Low;
        if !pressed {
            continue;
        }

        let is_running = !state.enabled.load(core::sync::atomic::Ordering::Relaxed);
        state
            .enabled
            .store(is_running, core::sync::atomic::Ordering::Relaxed);

        channel.enabled.publish(is_running).await;
    }
}
