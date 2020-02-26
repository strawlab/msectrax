#![no_main]
#![no_std]

// Tested hardware:
// F103RB, Nucleo-64 - everything works

/*

On F103RB at the time of writing, SPI was not working with using PB3 for SCK.
In particular, it seemed like things were working and the MOSI signal looked fine,
but there was no clock signal on PB3. I switched to PA5 and everything worked
and did not attempt to debug further. I guess somewhere in some initialization code PB3
is setup in an incompatible way. Update: I found
[this](https://github.com/japaric/stm32f103xx-hal/issues/116). I therefore disabled JTAG
below but have not tested if this frees PB3.

*/

// USART2:
// PA2 USART2_TX
// PA3 USART2_RX

// SPI:
// PA5 SPI1_SCK (Arduino D13) (also LD2 LED on Nucleo-64 board)
// PA6 SPI1_MISO (Arduino D12) (used, but not needed)
// PA7 SPI1_MOSI (Arduino D11)
// PB6 = data latch "A0" (Arduino D10)
// PC7 = update "A1" (Arduino D9)

// ADC:
// PC1 Analog adc1 (Arduino A4)
// PB1 Analog adc2 (no Arduino pin)

// Future ADC idea: perhaps switch to an ADC peripheral such as
// - ADS1602IPFBT (SPI)
// - LTC2335CLX-16#PBF (SPI)

extern crate panic_halt; // TODO better panic handling
extern crate stm32f1xx_hal as stm32_hal;

use stm32_hal::prelude::*;
use stm32_hal::serial::{self, Serial, Rx};
use stm32_hal::stm32::{USART2 as OtherUSART2, Interrupt};
use stm32_hal::spi::Spi;
use stm32_hal::gpio::{Output, PushPull};
use stm32_hal::gpio::gpioa::{PA5, PA6, PA7};
use stm32_hal::gpio::gpiob::PB6;
use stm32_hal::gpio::gpioc::PC7;

use stm32_hal::stm32::SPI1;
use stm32_hal::adc;

use embedded_hal::digital::v2::OutputPin;

use mini_rxtx::Decoded;

use msectrax_comms::{ToDevice, FromDevice, DeviceState, DeviceMode,
    ClosedLoopMode};
mod wrapped_tx;

// -----------------------

const BUFFER_SIZE: usize = 1200;

#[allow(dead_code)]
#[derive(Copy,Clone)]
pub struct StoredSample {
    adc1: u16,
    adc2: u16,
    dac1: u16,
    dac2: u16,
}

// -----------------------

#[derive(Debug)]
pub struct MyError {
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    SpiError(stm32_hal::spi::Error),
    UnitError,
}

impl From<stm32_hal::spi::Error> for ErrorKind {
    fn from(orig: stm32_hal::spi::Error) -> Self {
        ErrorKind::SpiError(orig)
    }
}

// impl From<()> for ErrorKind {
//     fn from(_orig: ()) -> Self {
//         ErrorKind::UnitError
//     }
// }

impl From<ErrorKind> for MyError {
    fn from(kind: ErrorKind) -> Self {
        Self{kind}
    }
}

impl From<()> for MyError {
    fn from(_orig: ()) -> Self {
        Self{kind: ErrorKind::UnitError}
    }
}

impl From<stm32_hal::spi::Error> for MyError {
    fn from(orig: stm32_hal::spi::Error) -> Self {
        Self{kind: ErrorKind::from(orig)}
    }
}

// -----------------------

pub type PushPullGpio = stm32_hal::gpio::Alternate<stm32_hal::gpio::PushPull>;

pub type FloatInput = stm32_hal::gpio::Input<stm32_hal::gpio::Floating>;

use dac714::Dac714Cascade;
pub type MyCascade = Dac714Cascade<Spi<SPI1,stm32_hal::spi::Spi1NoRemap,(PA5<PushPullGpio>, PA6<FloatInput>, PA7<PushPullGpio>)>, stm32_hal::spi::Error, PB6<Output<PushPull>>,PC7<Output<PushPull>>>;

// -----------------------

pub type ADC1ChType = stm32_hal::gpio::gpioc::PC1<stm32_hal::gpio::Analog>;
pub type ADC2ChType = stm32_hal::gpio::gpiob::PB1<stm32_hal::gpio::Analog>;

pub struct AnalogSystem {
    dev_adc1: stm32_hal::adc::Adc<stm32_hal::device::ADC1>,
    dev_adc2: stm32_hal::adc::Adc<stm32_hal::device::ADC2>,
    adc1: ADC1ChType,
    adc2: ADC2ChType,
}

// -----------------------

/// Calculate error angle based on the current ADC values and the calibration data
fn to_angle( p: &msectrax_comms::AdcToAngleCalibration, adc1: i16, adc2: i16 ) -> f32 {
    let adc1 = adc1 as f32;
    let adc2 = adc2 as f32;
    adc1*p.adc1_gain + adc2*p.adc2_gain + p.offset
}

fn query_adcs( dev_state: &mut DeviceState, analog: &mut AnalogSystem ) {
    let adc1: u16 = analog.dev_adc1.read(&mut analog.adc1).unwrap();
    let adc2: u16 = analog.dev_adc2.read(&mut analog.adc2).unwrap();

    dev_state.adc1 = adc1 as i16;
    dev_state.adc2 = adc2 as i16;
}

/// update galvos, returns true if galvo state changed
fn calculate_next_dac_values( dev_state: &mut DeviceState, cl_next_update_cycle: &mut u32) {

    // TODO FIXME: the timing here will not be very precise, since we update by
    // a fixed amount every time we are called. But since we are called from
    // idle(), it is not clear how often that will be, and it may be rather
    // variable.
    match &dev_state.inner.mode {
        DeviceMode::SawtoothTest => {
            dev_state.dac1 = dev_state.dac1.wrapping_add(10);
            dev_state.dac2 = dev_state.dac2.wrapping_add(20);
        }
        DeviceMode::SampleAdc => {}
        DeviceMode::ClosedLoop(cl_params) => {
            dev_state.cl_cycles = dev_state.cl_cycles.wrapping_add(1);

            if dev_state.cl_cycles == *cl_next_update_cycle {
                let azimuth_error = to_angle( &dev_state.inner.dac1_angle_func, dev_state.adc1, dev_state.adc2 ) - dev_state.inner.dac1_initial as f32;
                let elevation_error = to_angle( &dev_state.inner.dac2_angle_func, dev_state.adc1, dev_state.adc2 ) - dev_state.inner.dac2_initial as f32;

                match cl_params {
                    ClosedLoopMode::Proportional => {
                        dev_state.dac1_f32 += azimuth_error*dev_state.inner.dac1_angle_gain;
                        dev_state.dac2_f32 += elevation_error*dev_state.inner.dac2_angle_gain;
                    },
                }

                dev_state.dac1 = dev_state.dac1_f32 as i16;
                dev_state.dac2 = dev_state.dac2_f32 as i16;
                *cl_next_update_cycle = cl_next_update_cycle.wrapping_add(dev_state.inner.cl_period.get());
            }
        }
    }

    // clip dac values
    dev_state.dac1 = clip(dev_state.dac1,
        dev_state.inner.dac1_min,
        dev_state.inner.dac1_max);
    dev_state.dac2 = clip(dev_state.dac2,
        dev_state.inner.dac2_min,
        dev_state.inner.dac2_max);

}

fn delay_func() {
    // just do something to keep the CPU busy for a bit...
    let mut x: u16 = 0;
    for i in 0..65500 {
        x = x.wrapping_add(i);
    }
}

#[rtfm::app(device = stm32_hal::stm32, peripherals = true)]
const APP: () = {
    // Late resources
    struct Resources {
        rxtx: mini_rxtx::MiniTxRx<Rx<OtherUSART2>,wrapped_tx::WrappedTx>,
        state: DeviceState,
        cl_next_update_cycle: u32,
        dac714_cascade: MyCascade,
        // itm: cortex_m::peripheral::ITM,
        analog: AnalogSystem,
        ram_buffer: [StoredSample; BUFFER_SIZE],
    }

    #[init]
    fn init(c: init::Context) -> init::LateResources {

        // Device specific peripherals
        let device: stm32_hal::stm32::Peripherals = c.device;

        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();
        let gpio_bus = &mut rcc.apb2;

        let mut gpioa = device.GPIOA.split(gpio_bus);
        #[allow(unused_variables,unused_mut)]
        let mut gpiob = device.GPIOB.split(gpio_bus);
        #[allow(unused_variables,unused_mut)]
        let mut gpioc = device.GPIOC.split(gpio_bus);

        // let clocks = rcc.cfgr.freeze(&mut flash.acr);
        let clocks = rcc.cfgr
            .sysclk(64.mhz())
            .pclk1(32.mhz())
            .freeze(&mut flash.acr);

        // Setup ADC
        let dev_adc1 = adc::Adc::adc1(device.ADC1, &mut rcc.apb2, clocks);
        let dev_adc2 = adc::Adc::adc2(device.ADC2, &mut rcc.apb2, clocks);

        // Configure pc1, pb1 as an analog input
        let adc1 = gpioc.pc1.into_analog(&mut gpioc.crl);
        let adc2 = gpiob.pb1.into_analog(&mut gpiob.crl);
        // ADC stuff end ------

        // let cp = core;

        // // from https://github.com/rust-embedded/cortex-m/issues/82#issue-299792781
        // unsafe {
        //     use core::ptr;

        //     // enable TPIU and ITM
        //     cp.DCB.demcr.modify(|r| r | (1 << 24));

        //     // prescaler
        //     let swo_freq = 2_000_000;
        //     cp.TPIU.acpr.write((clocks.sysclk().0 / swo_freq) - 1);

        //     // SWO NRZ
        //     cp.TPIU.sppr.write(2);

        //     cp.TPIU.ffcr.modify(|r| r & !(1 << 1));

        //     // STM32 specific: enable tracing in the DBGMCU_CR register
        //     const DBGMCU_CR: *mut u32 = 0xe0042004 as *mut u32;
        //     let r = ptr::read_volatile(DBGMCU_CR);
        //     ptr::write_volatile(DBGMCU_CR, r | (1 << 5));

        //     // unlock the ITM
        //     cp.ITM.lar.write(0xC5ACCE55);

        //     cp.ITM.tcr.write(
        //         (0b000001 << 16) | // TraceBusID
        //         (1 << 3) | // enable SWO output
        //         (1 << 0), // enable the ITM
        //     );

        //     // enable stimulus port 0
        //     cp.ITM.ter[0].write(1);
        // }

        // // Cortex-M peripherals
        // let itm = cp.ITM;

        // initialize serial
        let tx = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);

        let rx = gpioa.pa3;

        // Prepare the alternate function I/O registers
        let mut afio = device.AFIO.constrain(&mut rcc.apb2);

        // Disable JTAG debugging, free PB3, PB4, PA15.
        afio.mapr.disable_jtag(gpioa.pa15, gpiob.pb3, gpiob.pb4);

        let mut serial = Serial::usart2(
            device.USART2,
            (tx, rx),
            &mut afio.mapr,
            // stm32_hal::time::Bps(msectrax_comms::BPS_HZ),
            stm32_hal::serial::Config::default().baudrate(msectrax_comms::BPS_HZ.bps()),
            clocks,
            &mut rcc.apb1,
        );

        serial.listen(serial::Event::Rxne);
        // serial.listen(serial::Event::Txe); // TODO I am confused why this is not needed.
        let (tx, rx) = serial.split();
        let state = DeviceState::default();

        // initialize dac714 cascade
        let cascade = {

            // initialize A0 pin for data latch
            let mut a0 = gpiob.pb6.into_push_pull_output(&mut gpiob.crl);
            a0.set_high().expect("a0 set high");

            // initialize A1 pin for update
            let mut a1 = gpioc.pc7.into_push_pull_output(&mut gpioc.crl);
            a1.set_high().expect("a1 set high");

            // initialize pins for SPI
            let sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);

            let miso = gpioa.pa6;

            let mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);

            // SPI rate
            let freq = 2.mhz();

            // create SPI device
            let spi = Spi::spi1(
                device.SPI1,
                (sck, miso, mosi),
                &mut afio.mapr,
                dac714::MODE,
                freq,
                clocks,
                &mut rcc.apb2,
            );

            // create DAC cascade
            let cascade = MyCascade::new(spi, a0, a1, delay_func).unwrap();

            cascade
        };

        let analog = AnalogSystem {
            dev_adc1,
            dev_adc2,
            adc1,
            adc2,
        };

        let cl_next_update_cycle = calc_next_update(&state);

        // Initialization of late resources
        init::LateResources {
            rxtx: mini_rxtx::MiniTxRx::new(wrapped_tx::WrappedTx{tx},rx),
            state,
            cl_next_update_cycle,
            dac714_cascade: cascade,
            // itm,
            analog,
            ram_buffer: [StoredSample {adc1: 0, adc2: 0, dac1: 0, dac2: 0}; BUFFER_SIZE],
        }
    }


    #[idle(resources = [rxtx, state, cl_next_update_cycle, analog, dac714_cascade])]
    fn idle(mut c: idle::Context) -> ! {

        // iprintln!(&mut resources.ITM.stim[0], "entered idle()");

        let mut decode_buf = [0u8; 256];
        let mut decoder = mini_rxtx::Decoder::new(&mut decode_buf);
        let mut encode_buf: [u8; 256] = [0; 256];

        loop {

            query_adcs( c.resources.state, c.resources.analog );

            calculate_next_dac_values( c.resources.state, c.resources.cl_next_update_cycle);

            c.resources.dac714_cascade.set_value_ab(
                c.resources.state.dac1, c.resources.state.dac2 ).unwrap();

            let maybe_byte = c.resources.rxtx.lock(|x| x.pump());
            if let Some(byte) = maybe_byte {

                // iprintln!(&c.resources.itm.stim[0], "got byte: {}", byte);

                // process byte
                let response = match decoder.consume::<msectrax_comms::ToDevice>(byte) {
                    Decoded::Msg(ToDevice::SetState(inner)) => {
                        let dac1 = inner.dac1_initial;
                        let dac2 = inner.dac2_initial;
                        let dac1_f32 = dac1 as f32;
                        let dac2_f32 = dac2 as f32;
                        let next_state = DeviceState {
                            inner,
                            cl_cycles: 0,
                            adc1: 0,
                            adc2: 0,
                            dac1,
                            dac2,
                            dac1_f32,
                            dac2_f32,
                        };
                        *(c.resources.cl_next_update_cycle) = calc_next_update(&next_state);
                        *(c.resources.state) = next_state;
                        Some(FromDevice::Empty)
                    },
                    Decoded::Msg(ToDevice::EchoRequest8(buf)) => {
                        Some(FromDevice::EchoResponse8(buf))
                    }
                    Decoded::Msg(ToDevice::QueryState) => {
                        Some(FromDevice::EchoState(c.resources.state.clone()))
                    }
                    Decoded::Msg(ToDevice::QueryAnalog) => {
                        let analog_state = (c.resources.state.adc1, c.resources.state.adc2);
                        Some(FromDevice::EchoAnalog(analog_state))
                    }
                    Decoded::Msg(ToDevice::QueryDatatypesVersion) => {
                        Some(FromDevice::EchoDatatypesVersion(msectrax_comms::DATATYPES_VERSION))
                    }
                    Decoded::Msg(ToDevice::SetGalvos((dac1,dac2))) => {
                        c.resources.state.inner.mode = DeviceMode::SampleAdc;
                        c.resources.state.dac1 = dac1;
                        c.resources.state.dac2 = dac2;
                        Some(FromDevice::Empty)
                    }
                    Decoded::FrameNotYetComplete => {
                        // Frame not complete yet, do nothing until next byte.
                        None
                    }
                    Decoded::Error(_) => {
                        panic!("error reading frame");
                    }
                };
                if let Some(response) = response {
                    let msg = mini_rxtx::serialize_msg(&response, &mut encode_buf).unwrap();
                    c.resources.rxtx.lock( |sender| {
                        sender.send_msg(msg).unwrap();
                    });

                    rtfm::pend(Interrupt::USART2);
                }

            } else {
                // TODO: fix things so we can do this. Right we busy-loop.
                // // no byte to process: go to sleep and wait for interrupt
                // cortex_m::asm::wfi();
            }
        }
    }

    // for F103
    #[task(binds = USART2, resources = [rxtx])]
    fn usart2(c: usart2::Context) {
        c.resources.rxtx.on_interrupt();
    }

};

fn calc_next_update(state: &DeviceState) -> u32 {
    state.cl_cycles + state.inner.cl_period.get()
}

fn clip<R>(cur: R, min: R, max: R) -> R
    where
        R: core::cmp::PartialOrd,
{
    if cur < min {
        return min;
    }
    if cur > max {
        return max;
    }
    cur
}
