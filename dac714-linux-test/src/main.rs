//! Raspberry Pi demo
//!
//! # Connections
//!
//! IMPORTANT: Do *not* use PIN24 / BCM8 / CE0 as the NCS pin
//!
//! - PIN1 = 3V3 = VCC
//! - PIN19 = BCM10 = MOSI (SDA)
//! - PIN21 = BCM9 = MISO (AD0)
//! - PIN23 = BCM11 = SCLK (SCL)
//! - PIN22 = BCM25 = NCS
//! - PIN6 = GND = GND

extern crate linux_embedded_hal as hal;
extern crate dac714;

use std::thread;
use std::time::Duration;

use hal::spidev::{self, SpidevOptions};
use hal::{Delay, Pin, Spidev};
use hal::sysfs_gpio::Direction;

fn main() {
    let mut spi = Spidev::open("/dev/spidev0.0").unwrap();
    let options = SpidevOptions::new()
        .max_speed_hz(1_000_000)
        .mode(spidev::SPI_MODE_3)
        .build();
    spi.configure(&options).unwrap();

    unimplemented!();

    // let ncs = Pin::new(25);
    // ncs.export().unwrap();
    // while !ncs.is_exported() {}
    // ncs.set_direction(Direction::Out).unwrap();
    // ncs.set_value(1).unwrap();

    thread::sleep(Duration::from_millis(100));

}
