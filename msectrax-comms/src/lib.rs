#![no_std]

#[macro_use]
extern crate serde_derive;
extern crate serde;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

pub const BPS_HZ: u32 = 115_200; // faster seems to work on linux, but not mac

pub const DATATYPES_VERSION: u16 = 5; // increment this when you change definitions below

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[repr(C)] // <--- required for ssmarshal
pub enum ToDevice {
    EchoRequest8((u8,u8,u8,u8,u8,u8,u8,u8)), // -> EchoResponse8
    SetState(SetDeviceState), // -> Empty
    QueryState, // -> EchoState
    QueryAnalog, // -> EchoAnalog
    QueryDatatypesVersion, // -> EchoDatatypesVersion,
    /// changes the device mode to SampleAdc
    SetGalvos((i16,i16)), // -> Empty
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[repr(C)] // <--- required for ssmarshal
pub enum FromDevice {
    EchoResponse8((u8,u8,u8,u8,u8,u8,u8,u8)),
    EchoState(DeviceState),
    EchoAnalog((i16,i16)),
    EchoDatatypesVersion(u16),
    Empty,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct SetDeviceState {
    pub mode: DeviceMode,
    pub cl_period: core::num::NonZeroU32,
    pub dac1_initial: i16,
    pub dac2_initial: i16,
    pub dac1_angle_func: AdcToAngleCalibration,
    pub dac2_angle_func: AdcToAngleCalibration,
    pub dac1_angle_gain: f32,
    pub dac2_angle_gain: f32,
    pub dac1_min: i16,
    pub dac1_max: i16,
    pub dac2_min: i16,
    pub dac2_max: i16,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct DeviceState {
    pub inner: SetDeviceState,
    pub cl_cycles: u32,
    pub adc1: i16,
    pub adc2: i16,
    pub dac1: i16,
    pub dac2: i16,
    pub dac1_f32: f32,
    pub dac2_f32: f32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[repr(C)] // <--- required for ssmarshal
pub enum ClosedLoopMode {
    Proportional,
    // ProportionalIntegral,
}

/// Result of calibration to calculate error angle from the adcs
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct AdcToAngleCalibration {
    pub adc1_gain: f32,
    pub adc2_gain: f32,
    pub offset: f32,
}

impl Default for SetDeviceState {
    fn default() -> Self {
        Self {
            mode: DeviceMode::default(),
            cl_period: core::num::NonZeroU32::new(1).unwrap(),
            dac1_initial: 0,
            dac2_initial: 0,
            dac1_angle_func: AdcToAngleCalibration::default(),
            dac2_angle_func: AdcToAngleCalibration::default(),
            dac1_angle_gain: 1e-3,
            dac2_angle_gain: 1e-3,
            dac1_min: i16::min_value(),
            dac1_max: i16::max_value(),
            dac2_min: i16::min_value(),
            dac2_max: i16::max_value(),
        }
    }
}


impl Default for DeviceState {
    fn default() -> Self {
        Self {
            inner: SetDeviceState::default(),
            cl_cycles: 0,
            adc1: 0,
            adc2: 0,
            dac1: 0,
            dac2: 0,
            dac1_f32: 0.0,
            dac2_f32: 0.0,
        }
    }
}

impl Default for AdcToAngleCalibration {
    fn default() -> Self {
        Self {
            adc1_gain: 0.1,
            adc2_gain: 0.1,
            offset: 0.0,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[repr(C)] // <--- required for ssmarshal
pub enum DeviceMode {
    SawtoothTest,
    SampleAdc,
    ClosedLoop(ClosedLoopMode),
}

impl Default for DeviceMode {
    fn default() -> Self {
        DeviceMode::SampleAdc
    }
}

// -----------------------------------------------------------------------------
// Testing stuff below here
// -----------------------------------------------------------------------------

#[cfg(test)]
impl quickcheck::Arbitrary for ToDevice {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        // use quickcheck::rand::Rng;
        use rand::{self, Rng};

        let rand: u8 = g.gen();
        let rem = rand % 3;
        match rem {
            0 => {
                ToDevice::EchoRequest8((g.gen(), g.gen(), g.gen(), g.gen(),
                    g.gen(), g.gen(), g.gen(), g.gen()))
            }
            1 => {
                let sds = SetDeviceState::arbitrary(g);
                ToDevice::SetState(sds)
            }
            2 => {
                ToDevice::QueryState
            }
            3 => {
                ToDevice::QueryAnalog
            }
            4 => {
                ToDevice::SetGalvos((g.gen(), g.gen()))
            }
            _ => {
                panic!("impossible");
            }
        }
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for SetDeviceState {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        use rand::{self, Rng};

        let mode = DeviceMode::arbitrary(g);
        Self {
            mode,
            cl_period: core::num::NonZeroU32::new(g.gen()).unwrap(),
            dac1_angle_func: AdcToAngleCalibration::arbitrary(g),
            dac2_angle_func: AdcToAngleCalibration::arbitrary(g),
            dac1_angle_gain: g.gen(),
            dac2_angle_gain: g.gen(),

            dac1_initial: g.gen(),
            dac2_initial: g.gen(),

            dac1_min: g.gen(),
            dac1_max: g.gen(),
            dac2_min: g.gen(),
            dac2_max: g.gen(),
        }
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for DeviceState {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        use rand::{self, Rng};

        let inner = SetDeviceState::arbitrary(g);
        Self {
            inner,
            cl_cycles: g.gen(),
            dac1: g.gen(),
            dac2: g.gen(),
            dac1_f32: g.gen(),
            dac2_f32: g.gen(),
            adc1: g.gen(),
            adc2: g.gen(),
        }
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for AdcToAngleCalibration {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        use rand::{self, Rng};

        Self {
            adc1_gain: g.gen(),
            adc2_gain: g.gen(),
            offset: g.gen(),
        }
    }
}


#[cfg(test)]
impl quickcheck::Arbitrary for DeviceMode {
    fn arbitrary<G: quickcheck::Gen>(_g: &mut G) -> Self {
        DeviceMode::SawtoothTest
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    quickcheck! {
        fn qc_to_device_ssmarshal_roundtrip(orig: crate::ToDevice) -> bool {
            let mut buf = [0; 256];
            let n_bytes = ssmarshal::serialize(&mut buf, &orig).expect("serialize");

            let (decoded, nbytes2) = ssmarshal::deserialize(&buf[0..n_bytes]).expect("deserialize");

            (orig == decoded) && (n_bytes == nbytes2)
        }
    }

    fn check_set_device_state(orig: &SetDeviceState) {
        let mut buf = [0; 256];
        let n_bytes = ssmarshal::serialize(&mut buf, orig)
            .expect("serialize");

        let (decoded, nbytes2): (SetDeviceState, _) = ssmarshal::deserialize(&buf[0..n_bytes])
            .expect("deserialize");

        assert_eq!(orig,&decoded);
        assert_eq!(n_bytes,nbytes2);
    }

    #[test]
    fn test_set_device_state_roundtrip() {

        // check default
        let mut dev_state = SetDeviceState::default();
        check_set_device_state(&dev_state);

        // check P controller
        dev_state.mode = DeviceMode::ClosedLoop(ClosedLoopMode::Proportional);
        check_set_device_state(&dev_state);

        // // check PI controller
        // dev_state.mode = DeviceMode::ClosedLoop(ClosedLoopMode::ProportionalIntegral);
        // check_set_device_state(&dev_state);
    }


    fn check_device_state(orig: &DeviceState) {
        let mut buf = [0; 256];
        let n_bytes = ssmarshal::serialize(&mut buf, orig)
            .expect("serialize");

        let (decoded, nbytes2): (DeviceState, _) = ssmarshal::deserialize(&buf[0..n_bytes])
            .expect("deserialize");

        assert_eq!(orig,&decoded);
        assert_eq!(n_bytes,nbytes2);
    }

    #[test]
    fn test_device_state_roundtrip() {
        // check default
        let dev_state = DeviceState::default();
        check_device_state(&dev_state);
    }

    #[test]
    fn test_errcase_roundtrip() {
        let orig = ToDevice::EchoRequest8((1, 2, 3, 4, 5, 6, 7, 8));

        let mut buf = [0; 32];
        let n_bytes = ssmarshal::serialize(&mut buf, &orig)
            .expect("serialize");

        let (decoded, nbytes2) = ssmarshal::deserialize(&buf[0..n_bytes])
            .expect("deserialize");

        assert_eq!(orig,decoded);
        assert_eq!(n_bytes,nbytes2);
    }

}
