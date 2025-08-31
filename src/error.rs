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

    /// Explorer-related errors
    Explorer(ExplorerError),

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
            ErrorKind::I2c(I2cError::Nack) => f.write_str("Nack"),
            ErrorKind::I2c(I2cError::ArbitrationLost) => f.write_str("ArbitrationLost"),
            ErrorKind::I2c(I2cError::Bus) => f.write_str("BusError"),
            ErrorKind::Spi(SpiError::ModeFault) => f.write_str("ModeFault"),
            ErrorKind::Gpio(GpioError::InvalidState) => f.write_str("InvalidState"),
            ErrorKind::Adc(AdcError::OutOfRange) => f.write_str("OutOfRange"),
            ErrorKind::Hardware(HardwareError::Power) => f.write_str("PowerFault"),
            ErrorKind::Hardware(HardwareError::Peripheral) => f.write_str("PeripheralFault"),
            ErrorKind::Hardware(HardwareError::ShortCircuit) => f.write_str("ShortCircuit"),
            ErrorKind::Hardware(HardwareError::OpenCircuit) => f.write_str("OpenCircuit"),
            ErrorKind::Buffer(BufferError::Overflow) => f.write_str("BufferOverflow"),
            ErrorKind::Buffer(BufferError::Underflow) => f.write_str("BufferUnderflow"),
            ErrorKind::InvalidConfig => f.write_str("InvalidConfig"),
            ErrorKind::Unknown => f.write_str("Unknown"),
            ErrorKind::Other => f.write_str("Other"),
            ErrorKind::Explorer(e) => write!(f, "Explorer: {}", e),
        }
    }
}

/// Errors that can occur during exploration of command sequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerError {
    /// The provided sequence contained more commands than supported by the capacity N.
    TooManyCommands,
    /// The command dependency graph contains a cycle.
    DependencyCycle,
    /// No valid I2C addresses were found for any command sequence.
    NoValidAddressesFound,
    /// An I2C command execution failed.
    ExecutionFailed,
    /// An internal buffer overflowed.
    BufferOverflow,
    /// A dependency index is out of bounds.
    InvalidDependencyIndex,
    /// An I2C scan operation failed.
    DeviceNotFound,
}

/// Errors that can occur during command execution.
#[derive(Debug, PartialEq, Eq)]
pub enum ExecutorError {
    /// The command failed to execute due to an I2C error.
    I2cError(crate::error::ErrorKind),
    /// The command failed to execute (e.g., NACK, I/O error).
    ExecFailed,
    /// An internal buffer overflowed during command preparation.
    BufferOverflow,
}

impl From<ExecutorError> for ExplorerError {
    fn from(error: ExecutorError) -> Self {
        match error {
            ExecutorError::I2cError(_) => ExplorerError::ExecutionFailed,
            ExecutorError::ExecFailed => ExplorerError::ExecutionFailed,
            ExecutorError::BufferOverflow => ExplorerError::BufferOverflow,
        }
    }
}

impl fmt::Display for ExplorerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExplorerError::TooManyCommands => f.write_str("TooManyCommands"),
            ExplorerError::DependencyCycle => f.write_str("DependencyCycle"),
            ExplorerError::NoValidAddressesFound => f.write_str("NoValidAddressesFound"),
            ExplorerError::ExecutionFailed => f.write_str("ExecutionFailed"),
            ExplorerError::BufferOverflow => f.write_str("BufferOverflow"),
            ExplorerError::InvalidDependencyIndex => f.write_str("InvalidDependencyIndex"),
            ExplorerError::DeviceNotFound => f.write_str("DeviceNotFound"),
        }
    }
}
