use stm32_hal::serial::Tx;
use stm32_hal::stm32::USART2;

pub struct WrappedTx {
    pub(crate) tx: Tx<USART2>,
}

impl embedded_hal::serial::Write<u8> for WrappedTx {
    type Error = core::convert::Infallible;
    #[inline]
    fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
        self.tx.write(byte)
    }
    #[inline]
    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        self.tx.flush()
    }
}
