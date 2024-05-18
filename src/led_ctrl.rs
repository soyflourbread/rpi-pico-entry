use crate::{BRIGHTNESS_THRESHOLD, ENABLED};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Sender;
use embassy_sync::pubsub::Subscriber;
use embassy_time::{Duration, Ticker};

pub struct Config {
    pub id: usize,

    pub k: u16,
    pub multiplier: f32,

    pub tick: Duration,
}

pub struct Channel {
    pub running: Subscriber<'static, ThreadModeRawMutex, bool, 4, 4, 4>,
    pub led: Sender<'static, ThreadModeRawMutex, (usize, u16), 64>,
}

#[embassy_executor::task(pool_size = 2)]
pub async fn run(config: Config, mut channel: Channel) {
    let mut ticker = Ticker::every(config.tick);
    'outer: loop {
        while !ENABLED.load(core::sync::atomic::Ordering::Relaxed) {
            // not running, better start waiting
            channel.led.send((config.id, u16::MIN)).await;
            channel.running.next_message_pure().await;
        }
        ticker.reset();

        loop {
            let mut brightness = config.k as f32;
            while (brightness as u16) < BRIGHTNESS_THRESHOLD {
                if !ENABLED.load(core::sync::atomic::Ordering::Relaxed) {
                    continue 'outer;
                }
                channel.led.send((config.id, brightness as u16)).await;
                brightness *= config.multiplier;
                ticker.next().await;
            }
            brightness /= config.multiplier;
            while (brightness as u16) >= config.k {
                if !ENABLED.load(core::sync::atomic::Ordering::Relaxed) {
                    continue 'outer;
                }
                channel.led.send((config.id, brightness as u16)).await;
                brightness /= config.multiplier;
                ticker.next().await;
            }
        }
    }
}
