pub enum ErrorKind {
    UartFraming,
    UartParity,
    UartOverrun,
    UartUnderrun,
    UartTimeout,

    BusNack,
    BusArbitrationLost,
    Bus,

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
