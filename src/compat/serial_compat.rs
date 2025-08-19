use core::fmt::Debug;
use nb; // nbクレートを使用

/// common Serial Write trait
pub trait SerialCompat {
    type Error: Debug;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error>;
    fn flush(&mut self) -> Result<(), Self::Error>;
}

// ========== ehal 0.2.x ==========
#[cfg(all(feature = "ehal_0_2", not(feature = "ehal_1_0")))]
impl<SERIAL> SerialCompat for SERIAL
where
    SERIAL: embedded_hal_0_2::serial::Write<u8>,
    <SERIAL as embedded_hal_0_2::serial::Write<u8>>::Error: Debug + Copy,
{
    type Error = <SERIAL as embedded_hal_0_2::serial::Write<u8>>::Error;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        nb::block!(embedded_hal_0_2::serial::Write::write(self, byte))
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        nb::block!(embedded_hal_0_2::serial::Write::flush(self))
    }
}

// ========== ehal 1.0 ==========
#[cfg(feature = "ehal_1_0")]
impl<SERIAL> SerialCompat for SERIAL
where
    SERIAL: embedded_hal_1::serial::nb::Write<u8>,
    <SERIAL as embedded_hal_1::serial::nb::Write<u8>>::Error: Debug + Copy,
{
    type Error = <SERIAL as embedded_hal_1::serial::nb::Write<u8>>::Error;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        nb::block!(embedded_hal_1::serial::nb::Write::write(self, byte))
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        nb::block!(embedded_hal_1::serial::nb::Write::flush(self))
    }
}
