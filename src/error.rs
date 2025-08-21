#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// UART-related errors
    Uart(UartError),
    /// I2C-related errors
    I2c(I2cError),
    /// SPI-related errors
    Spi(SpiError),
    /// GPIO-related errors
    Gpio(GpioError),
    /// ADC-related errors
    Adc(AdcError),
    /// Hardware-level faults (power, short, etc.)
    Hardware(HardwareError),
    /// Buffer / data structure related errors
    Buffer(BufferError),

    /// Invalid configuration or unsupported setup
    InvalidConfig,

    /// Unknown error (cannot be categorized)
    Unknown,

    /// Other errors (external/custom)
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UartError {
    Framing,
    Parity,
    Overrun,
    Underrun,
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cError {
    Nack,
    ArbitrationLost,
    Bus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiError {
    ModeFault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioError {
    InvalidState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdcError {
    OutOfRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareError {
    Power,
    Peripheral,
    ShortCircuit,
    OpenCircuit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferError {
    Overflow,
    Underflow,
}
