#[derive(Debug, Clone, Copy, PartialEq)] // Clone, Copy, PartialEqを追加
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
