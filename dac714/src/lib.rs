#![no_std]

// Rust code based on:
// - https://github.com/japaric/mfrc522
// - https://github.com/japaric/mpu9250
// - https://github.com/japaric/l3gd20 (usage in STM32F30x: https://github.com/japaric/f3/blob/master/examples/l3gd20.rs)

// SPI code based on
// - https://github.com/strawlab/flymad/blob/master/flymad_micro/v2/dac714.h

// Note: in theory, it should be possible to use this crate with the spidev
// driver on Raspberry Pi. See, for example,
// https://github.com/japaric/mpu9250/blob/master/examples/rpi.rs

use embedded_hal::blocking::spi;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi::{Mode, Phase, Polarity};

use byteorder::ByteOrder;

#[derive(Debug)]
pub struct D7Error {
    kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    SpiError,
    OutputPinError,
}

impl From<ErrorKind> for  D7Error {
    fn from(kind: ErrorKind) -> Self {
        Self {kind}
    }
}

/// SPI mode
pub const MODE: Mode = Mode {
    phase: Phase::CaptureOnSecondTransition,
    polarity: Polarity::IdleLow,
};

/// One, two or three DAC714 chips with cascaded synchronous operation.
///
/// The circuit should be wired according to "FIGURE 8a. Cascaded Serial Bus
/// Connection with Synchronous Update" in the [DAC714
/// datasheet](https://www.ti.com/lit/ds/symlink/dac714.pdf).
///
/// Connections
/// - A0 = Data latch
/// - A1 = Update
/// - SPI = SPI
pub struct Dac714Cascade<SPI, SpiErr, A0, A1>
where
    SPI: spi::Write<u8, Error = SpiErr> + spi::Transfer<u8, Error = SpiErr>,
    A0: OutputPin,
    A1: OutputPin,
{
    spi: SPI,
    a0: A0,
    a1: A1,
    delay_func: fn(),
}

fn new<SPI, SpiErr, A0, A1>(
    spi: SPI,
    a0: A0,
    a1: A1,
    delay_func: fn(),
) -> Result<Dac714Cascade<SPI, SpiErr, A0, A1>, D7Error>
where
    SPI: spi::Write<u8, Error = SpiErr> + spi::Transfer<u8, Error = SpiErr>,
    A0: OutputPin,
    A1: OutputPin,
{
    let dac714 = Dac714Cascade {
        spi,
        a0,
        a1,
        delay_func,
    };

    Ok(dac714)
}

impl<SPI, SpiErr, A0, A1> Dac714Cascade<SPI, SpiErr, A0, A1>
where
    SPI: spi::Write<u8, Error = SpiErr> + spi::Transfer<u8, Error = SpiErr>,
    A0: OutputPin,
    A1: OutputPin,
{
    pub fn new(spi: SPI, a0: A0, a1: A1, delay_func: fn()) -> Result<Self, D7Error>
    {
        new(spi, a0, a1, delay_func)
    }

    /// Destroys the driver recovering the SPI peripheral and the pins
    pub fn release(self) -> (SPI, A0, A1) {
        (self.spi, self.a0, self.a1)
    }

    fn write_buf(&mut self, buf: &[u8])  -> Result<(),D7Error> {
        self.a0.set_low().map_err(|_e| D7Error::from(ErrorKind::OutputPinError))?;
        self.spi.write(buf).map_err(|_e| D7Error::from(ErrorKind::SpiError))?;
        self.a0.set_high().map_err(|_e| D7Error::from(ErrorKind::OutputPinError))?;

        self.a1.set_low().map_err(|_e| D7Error::from(ErrorKind::OutputPinError))?;
        (self.delay_func)();
        self.a1.set_high().map_err(|_e| D7Error::from(ErrorKind::OutputPinError))?;
        Ok(())
    }

    /// Set DAC A
    pub fn set_value_a(&mut self, a: i16) -> Result<(),D7Error> {
        let mut buf = [0u8; 2];

        byteorder::BigEndian::write_i16(&mut buf[0..2], a);
        self.write_buf(&buf)
    }

    /// Set DAC A,B
    pub fn set_value_ab(&mut self, a: i16, b: i16) -> Result<(),D7Error> {
        let mut buf = [0u8; 4];

        byteorder::BigEndian::write_i16(&mut buf[0..2], a);
        byteorder::BigEndian::write_i16(&mut buf[2..4], b);
        self.write_buf(&buf)
    }

    /// Set DAC A,B,C
    pub fn set_value_abc(&mut self, a: i16, b: i16, c: i16) -> Result<(),D7Error> {
        let mut buf = [0u8; 6];

        byteorder::BigEndian::write_i16(&mut buf[0..2], a);
        byteorder::BigEndian::write_i16(&mut buf[2..4], b);
        byteorder::BigEndian::write_i16(&mut buf[4..6], c);
        self.write_buf(&buf)
    }

}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_byteorder_endian_coding() {
        // Test that the byteorder::BigEndian coding is consistent with the
        // description in the DAC714 datasheet. "The DAC714 is designed to
        // accept binary two’s comple- ment (BTC) input codes with the MSB first
        // which are compatible with bipolar analog output operation. For this
        // configuration, a digital input of 7FFFH produces a plus full scale
        // output, 8000H produces a minus full scale output, and 0000H produces
        // bipolar zero output."

        let full_plus_buf: [u8; 2] = [0x7f, 0xff];
        let full_minus_buf: [u8; 2] = [0x80, 0x00];
        let zero_buf: [u8; 2] = [0x00, 0x00];

        assert_eq!( 0i16, byteorder::BigEndian::read_i16(&zero_buf));
        assert_eq!( i16::max_value(), byteorder::BigEndian::read_i16(&full_plus_buf));
        assert_eq!( i16::min_value(), byteorder::BigEndian::read_i16(&full_minus_buf));
    }
}
