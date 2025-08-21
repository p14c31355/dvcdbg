#[derive(Debug)]
pub enum ErrorKind {
    UartFraming,
    UartParity,
    UartOverrun,
    UartUnderrun,
    UartTimeout,

    I2cNack,
    I2cArbitrationLost,
    I2cBus,

    HardPower,
    HardPeripheral,
    HardShortCircuit,
    HardOpenCircuit,

    BufferOverflow,
    BufferUnderflow,
    InvalidConfig,

    SpiModeFault,
    GpioInvalidState,
    AdcOutOfRange,
    Unknown,

    Other,

}
