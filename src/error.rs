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

use core::fmt;

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Uart(UartError::Framing) => f.write_str("UartFraming"),
            ErrorKind::Uart(UartError::Parity) => f.write_str("UartParity"),
            ErrorKind::Uart(UartError::Overrun) => f.write_str("UartOverrun"),
            ErrorKind::Uart(UartError::Underrun) => f.write_str("UartUnderrun"),
            ErrorKind::Uart(UartError::Timeout) => f.write_str("UartTimeout"),
            ErrorKind::I2c(I2cError::Nack) => f.write_str("NACK"),
            ErrorKind::I2c(I2cError::ArbitrationLost) => f.write_str("Arbitration_Lost"),
            ErrorKind::I2c(I2cError::Bus) => f.write_str("Bus_Error"),
            ErrorKind::Spi(SpiError::ModeFault) => f.write_str("Mode_Fault"),
            ErrorKind::Gpio(GpioError::InvalidState) => f.write_str("Invalid_State"),
            ErrorKind::Adc(AdcError::OutOfRange) => f.write_str("Out_Of_Range"),
            ErrorKind::Hardware(HardwareError::Power) => f.write_str("Power_Fault"),
            ErrorKind::Hardware(HardwareError::Peripheral) => f.write_str("Peripheral_Fault"),
            ErrorKind::Hardware(HardwareError::ShortCircuit) => f.write_str("Short_Circuit"),
            ErrorKind::Hardware(HardwareError::OpenCircuit) => f.write_str("Open_Circuit"),
            ErrorKind::Buffer(BufferError::Overflow) => f.write_str("Buffer_Overflow"),
            ErrorKind::Buffer(BufferError::Underflow) => f.write_str("Buffer_Underflow"),
            ErrorKind::InvalidConfig => f.write_str("Invalid_Config"),
            ErrorKind::Unknown => f.write_str("Unknown"),
            ErrorKind::Other => f.write_str("OTHER"),
            _ => f.write_str("..."),
        }
    }
}