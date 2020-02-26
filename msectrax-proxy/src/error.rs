#[derive(Debug)]
pub enum Error {
    MiniRxTxError(mini_rxtx::Error),
    Io(std::io::Error),
    ParseInt(std::num::ParseIntError),
    Serial(serialport::Error),
    CrossbeamSend(crossbeam_channel::SendError<msectrax_comms::ToDevice>),
    CrossbeamRecvTimeout(crossbeam_channel::RecvTimeoutError),
    SerialStart,
    FirmwareVersionMismatch((u16,u16)),
    FirmwareVersionCheckTimeout,
}

impl From<std::io::Error> for Error {
    fn from(orig: std::io::Error) -> Error {
        Error::Io(orig)
    }
}

impl From<mini_rxtx::Error> for Error {
    fn from(orig: mini_rxtx::Error) -> Error {
        Error::MiniRxTxError(orig)
    }
}

impl From<serialport::Error> for Error {
    fn from(orig: serialport::Error) -> Error {
        Error::Serial(orig)
    }
}

impl From<crossbeam_channel::SendError<msectrax_comms::ToDevice>> for Error {
    fn from(orig: crossbeam_channel::SendError<msectrax_comms::ToDevice>) -> Error {
        Error::CrossbeamSend(orig)
    }
}

impl From<crossbeam_channel::RecvTimeoutError> for Error {
    fn from(orig: crossbeam_channel::RecvTimeoutError) -> Error {
        Error::CrossbeamRecvTimeout(orig)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(orig: std::num::ParseIntError) -> Error {
        Error::ParseInt(orig)
    }
}
