use byteorder::ByteOrder;

pub enum Decoded<T> {
    Msg(T),
    FrameNotYetComplete,
    Error(crate::Error),
}


/// A struct for decoding bytes.
///
/// This is similar to `Decoder` but uses `std` to allocate a buffer.
///
/// This is not part of the `MiniTxRx` struct itself because we do not want to
/// require access to resources when decoding bytes.
#[cfg(feature="std")]
pub struct StdDecoder {
    buf: Vec<u8>,
    state: FramedReaderState,
}

#[cfg(feature="std")]
impl StdDecoder {
    pub fn new(sz: usize) -> Self {
        Self {
            buf: vec![0; sz],
            state: FramedReaderState::Empty,
        }
    }

    pub fn consume<T>(&mut self, byte: u8) -> Decoded<T>
        where
            for<'de> T: serde::de::Deserialize<'de>,
    {
        let (new_state, decoded) = consume_inner(&mut self.state, &mut self.buf, byte);
        self.state = new_state;
        decoded
    }
}

/// A struct for decoding bytes.
///
/// This is not part of the `MiniTxRx` struct itself because we do not want to
/// require access to resources when decoding bytes.
pub struct Decoder<'a> {
    buf: &'a mut [u8],
    state: FramedReaderState,
}

impl<'a> Decoder<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self {
            buf,
            state: FramedReaderState::Empty,
        }
    }

    pub fn consume<T>(&mut self, byte: u8) -> Decoded<T>
        where
            for<'de> T: serde::de::Deserialize<'de>,
    {
        let (new_state, decoded) = consume_inner(&mut self.state, &mut self.buf, byte);
        self.state = new_state;
        decoded
    }
}

#[inline]
fn consume_inner<T>(self_state: &mut FramedReaderState, self_buf: &mut[u8], byte: u8) -> (FramedReaderState, Decoded<T>)
    where
        for<'de> T: serde::de::Deserialize<'de>,
{
    let (new_state, result) = match self_state {
        FramedReaderState::Empty => (FramedReaderState::ReadingHeader(byte), Ok(None)),
        FramedReaderState::ReadingHeader(byte0) => {
            let buf: [u8; 2] = [*byte0, byte];
            let len = ::byteorder::LittleEndian::read_u16(&buf);
            if (len as usize) > self_buf.len() {
                (FramedReaderState::Error, Err(crate::Error::TooLong))
            } else {
                let rms = ReadingMessageState { len: len, idx: 0 };
                (FramedReaderState::ReadingMessage(rms), Ok(None))
            }
        }
        FramedReaderState::ReadingMessage(ref rms) => {
            let (msg_len, mut idx) = (rms.len, rms.idx);
            self_buf[idx as usize] = byte;
            idx += 1;
            if idx < msg_len {
                let rms = ReadingMessageState {
                    len: msg_len,
                    idx: idx,
                };
                (FramedReaderState::ReadingMessage(rms), Ok(None))
            } else if idx == msg_len {
                let result = &self_buf[0..(idx as usize)];
                (FramedReaderState::Empty, Ok(Some(result)))
            } else {
                // Frame langer than expected.
                // Theoretically it is impossible to get here, so we panic.
                panic!("frame larger than expected");
            }
        }
        FramedReaderState::Error => (FramedReaderState::Error, Err(crate::Error::PreviousError)),
    };
    let decoded = match result {
        Ok(Some(buf)) => {
            match ssmarshal::deserialize(buf) {
                Ok((msg, _nbytes)) => Decoded::Msg(msg),
                Err(e) => Decoded::Error(e.into()),
            }
        },
        Ok(None) => {
            Decoded::FrameNotYetComplete
        },
        Err(e) => {
            Decoded::Error(e)
        }
    };
    (new_state, decoded)
}

struct ReadingMessageState {
    len: u16, // the length when full
    idx: u16, // the current length
}

enum FramedReaderState {
    Empty,
    ReadingHeader(u8),
    ReadingMessage(ReadingMessageState),
    Error,
}
