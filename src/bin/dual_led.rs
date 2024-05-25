#![no_std]
#![no_main]

use rpi_pico_entry::{led_ctrl, status, toggle};

use core::sync::atomic::AtomicBool;

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{gpio, pwm, uart, adc};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::pubsub::PubSubChannel;
use embassy_time::Duration;

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
        threshold: BRIGHTNESS_THRESHOLD,
        tick,
    };
    let config_1 = led_ctrl::Config {
        id: 1,
        k: 8,
        multiplier: 1.1,
        threshold: BRIGHTNESS_THRESHOLD,
        tick,
    };
    let to_state = || led_ctrl::State { enabled: &ENABLED };
    let to_channel = || led_ctrl::Channel {
        enabled: CHANNEL_ENABLED.subscriber().unwrap(),
        led: CHANNEL_LED.sender(),
    };
    unwrap!(spawner.spawn(led_ctrl::run(config_0, to_state(), to_channel())));
    unwrap!(spawner.spawn(led_ctrl::run(config_1, to_state(), to_channel())));
}

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

embassy_rp::bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => adc::InterruptHandler;
});

fn to_celsius(raw_temp: u16) -> f32 {
    // According to chapter 4.9.5. Temperature Sensor in RP2040 datasheet
    let temp = 27.0 - (raw_temp as f32 * 3.3 / 4096.0 - 0.706) / 0.001721;
    let sign = if temp < 0.0 { -1.0 } else { 1.0 };
    let rounded_temp_x10: i16 = ((temp * 10.0) + 0.5 * sign) as i16;
    (rounded_temp_x10 as f32) / 10.0
}

fn to_voltage(raw_voltage: u16) -> f32 {
    let mut ret = raw_voltage as f32;
    ret *= 3.3;
    ret *= 3.0;
    ret /= 65535.0;
    ret
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut adc_0 = adc::Adc::new(p.ADC, Irqs, adc::Config::default());
    let mut ts = adc::Channel::new_temp_sensor(p.ADC_TEMP_SENSOR);
    let temp = adc_0.read(&mut ts).await.unwrap();
    let temp = to_celsius(temp);
    info!("board temperature: {} C", temp);

    let mut p_voltage = adc::Channel::new_pin(p.PIN_29, gpio::Pull::None);
    let voltage = adc_0.read(&mut p_voltage).await.unwrap();
    let voltage = to_voltage(voltage);
    info!("input voltage: {}", voltage);

    let mut uart_0 = uart::Uart::new_blocking(p.UART0, p.PIN_16, p.PIN_17, uart::Config::default());
    uart_0
        .blocking_write("Hello World!\r\n".as_bytes())
        .unwrap();
    info!("board ok, spinning up state machines");

    let mut pwm_0_config: pwm::Config = Default::default();
    pwm_0_config.top = 0x8000;
    pwm_0_config.compare_a = 8;
    pwm_0_config.compare_b = 8;
    let mut pwm_0 = pwm::Pwm::new_output_ab(p.PWM_SLICE0, p.PIN_0, p.PIN_1, pwm_0_config.clone());

    status_run(&spawner, gpio::AnyPin::from(p.PIN_25));
    led_ctrl_run(&spawner);
    toggle_run(&spawner, gpio::AnyPin::from(p.PIN_22));
    info!("setup done");

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
