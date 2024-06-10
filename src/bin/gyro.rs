#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::{adc, bind_interrupts, gpio};
use embassy_rp::{i2c, peripherals, uart};
use embassy_time::{Duration, Ticker, Timer};

use defmt::{info, unwrap};
use heapless::String;
use core::fmt::Write;

use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => adc::InterruptHandler;
    I2C0_IRQ => i2c::InterruptHandler<peripherals::I2C0>;
});

fn convert_to_celsius(raw_temp: u16) -> f32 {
    // According to chapter 4.9.5. Temperature Sensor in RP2040 datasheet
    let temp = 27.0 - (raw_temp as f32 * 3.3 / 4096.0 - 0.706) / 0.001721;
    let sign = if temp < 0.0 { -1.0 } else { 1.0 };
    let rounded_temp_x10: i16 = ((temp * 10.0) + 0.5 * sign) as i16;
    (rounded_temp_x10 as f32) / 10.0
}

fn to_voltage(raw_voltage: u16) -> f32 {
    let mut ret = raw_voltage as f32;
    ret *= 3.23;
    ret *= 3.0;
    ret /= 4096.0;
    ret
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut uart_0_cfg = uart::Config::default();
    uart_0_cfg.baudrate = 9600;
    let mut uart_0 = uart::Uart::new_blocking(p.UART0, p.PIN_16, p.PIN_17, uart_0_cfg);
    uart_0
        .blocking_write("Hello World!\r\n".as_bytes())
        .unwrap();
    info!("board ok");

    let mut adc = adc::Adc::new(p.ADC, Irqs, adc::Config::default());
    let mut adc_temp = adc::Channel::new_temp_sensor(p.ADC_TEMP_SENSOR);
    let mut adc_voltage = adc::Channel::new_pin(p.PIN_29, gpio::Pull::None);

    let mut i2c_0 = i2c::I2c::new_async(p.I2C0, p.PIN_1, p.PIN_0, Irqs, i2c::Config::default());

    let mut ticker = Ticker::every(Duration::from_millis(400));
    
    while let () = ticker.next().await {
        let voltage = adc.read(&mut adc_voltage).await.unwrap();
        let voltage = to_voltage(voltage);
        
        let Ok(rp_temp) = adc.read(&mut adc_temp).await else { continue; };
        let rp_temp = convert_to_celsius(rp_temp);
        
        let mut frame = String::<128>::new();
        let _ = write!(frame, "rp_vol: {}, rp_temp: {} \r\n", voltage, rp_temp);
        uart_0
            .blocking_write(frame.as_bytes())
            .unwrap();
    }
}
