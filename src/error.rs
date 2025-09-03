use core::fmt;

/// Defines the category of an error.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Errors related to the UART peripheral.
    Uart(UartError),
    /// Errors related to the I2C peripheral.
    I2c(I2cError),
    /// Errors related to the SPI peripheral.
    Spi(SpiError),
    /// Errors related to the GPIO peripheral.
    Gpio(GpioError),
    /// Errors related to the ADC peripheral.
    Adc(AdcError),
    /// Hardware-level faults such as power, short circuits, etc.
    Hardware(HardwareError),
    /// Errors related to buffers or data structures.
    Buffer(BufferError),
    /// An invalid configuration or an unsupported setup.
    InvalidConfig,
    /// An unknown error that cannot be categorized.
    Unknown,
    /// Other external or custom errors.
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UartError {
    /// A framing error occurred.
    Framing,
    /// A parity error occurred.
    Parity,
    /// An overrun error occurred.
    Overrun,
    /// An underrun error occurred.
    Underrun,
    /// A timeout occurred during an operation.
    Timeout,
}

impl fmt::Display for UartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UartError::Framing => f.write_str("Framing"),
            UartError::Parity => f.write_str("Parity"),
            UartError::Overrun => f.write_str("Overrun"),
            UartError::Underrun => f.write_str("Underrun"),
            UartError::Timeout => f.write_str("Timeout"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum I2cError {
    /// A NACK (No Acknowledgment) was received from a device.
    Nack,
    /// Arbitration was lost during an I2C transaction.
    ArbitrationLost,
    /// A general bus error occurred.
    Bus,
}

impl fmt::Display for I2cError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            I2cError::Nack => f.write_str("Nack"),
            I2cError::ArbitrationLost => f.write_str("ArbitrationLost"),
            I2cError::Bus => f.write_str("Bus"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SpiError {
    /// A mode fault occurred on the SPI bus.
    ModeFault,
}

impl fmt::Display for SpiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpiError::ModeFault => f.write_str("ModeFault"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GpioError {
    /// An invalid state was detected for a GPIO pin.
    InvalidState,
}

impl fmt::Display for GpioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpioError::InvalidState => f.write_str("InvalidState"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AdcError {
    /// The ADC reading is out of its valid range.
    OutOfRange,
}

impl fmt::Display for AdcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdcError::OutOfRange => f.write_str("OutOfRange"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HardwareError {
    /// A power fault was detected.
    Power,
    /// A peripheral-related fault occurred.
    Peripheral,
    /// A short circuit was detected.
    ShortCircuit,
    /// An open circuit was detected.
    OpenCircuit,
}

impl fmt::Display for HardwareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HardwareError::Power => f.write_str("PowerFault"),
            HardwareError::Peripheral => f.write_str("PeripheralFault"),
            HardwareError::ShortCircuit => f.write_str("ShortCircuit"),
            HardwareError::OpenCircuit => f.write_str("OpenCircuit"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BufferError {
    /// A buffer overflow occurred.
    Overflow,
    /// A buffer underflow occurred.
    Underflow,
}

impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BufferError::Overflow => f.write_str("Overflow"),
            BufferError::Underflow => f.write_str("Underflow"),
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Uart(e) => write!(f, "Uart: {e}"),
            ErrorKind::I2c(e) => write!(f, "I2c: {e}"),
            ErrorKind::Spi(e) => write!(f, "Spi: {e}"),
            ErrorKind::Gpio(e) => write!(f, "Gpio: {e}"),
            ErrorKind::Adc(e) => write!(f, "Adc: {e}"),
            ErrorKind::Hardware(e) => write!(f, "Hardware: {e}"),
            ErrorKind::Buffer(e) => write!(f, "Buffer: {e}"),
            ErrorKind::InvalidConfig => f.write_str("InvalidConfig"),
            ErrorKind::Unknown => f.write_str("Unknown"),
            ErrorKind::Other => f.write_str("Other"),
        }
    }
}

/// Errors that can occur within the BitFlags utility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitFlagsError {
    /// An index is out of bounds for the bit flags.
    IndexOutOfBounds { idx: usize, max: usize },
}

impl fmt::Display for BitFlagsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BitFlagsError::IndexOutOfBounds { idx, max } => {
                write!(
                    f,
                    "Index out of bounds: index {idx} is out of range 0..{max}"
                )
            }
        }
    }
}

/// Errors that can occur during the exploration of command sequences.
#[derive(PartialEq, Eq)]
pub enum ExplorerError {
    /// The provided sequence contained more commands than supported by the capacity.
    TooManyCommands,
    /// The command dependency graph contains a cycle.
    DependencyCycle,
    /// No valid I2C addresses were found for any command sequence.
    NoValidAddressesFound,
    /// An I2C command execution failed.
    ExecutionFailed(ErrorKind),
    /// An internal buffer overflowed during the exploration process.
    BufferOverflow,
    /// A dependency index is out of bounds.
    InvalidDependencyIndex,
    /// An I2C device was not found during a scan operation.
    DeviceNotFound(ErrorKind),
    /// An error occurred in the BitFlags utility.
    BitFlags(BitFlagsError),
}

impl fmt::Display for ExplorerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExplorerError::TooManyCommands => f.write_str("TooManyCommands"),
            ExplorerError::DependencyCycle => f.write_str("DependencyCycle"),
            ExplorerError::NoValidAddressesFound => f.write_str("NoValidAddressesFound"),
            ExplorerError::ExecutionFailed(kind) => write!(f, "ExecutionFailed: {kind}"),
            ExplorerError::BufferOverflow => f.write_str("BufferOverflow"),
            ExplorerError::InvalidDependencyIndex => f.write_str("InvalidDependencyIndex"),
            ExplorerError::DeviceNotFound(kind) => write!(f, "DeviceNotFound: {kind}"),
            ExplorerError::BitFlags(e) => write!(f, "BitFlagsError: {e}"),
        }
    }
}

/// Errors that can occur during command execution.
#[derive(PartialEq, Eq)]
pub enum ExecutorError {
    /// A command failed to execute due to an I2C error.
    I2cError(ErrorKind),
    /// The command failed to execute (e.g., NACK, I/O error).
    ExecFailed,
    /// An internal buffer overflowed during command preparation.
    BufferOverflow,
    /// An error occurred in the BitFlags utility.
    BitFlags(BitFlagsError),
    /// An error occurred in the explorer module.
    Explorer(ExplorerError),
}

/// Converts an `ExecutorError` into an `ExplorerError`.
impl From<ExecutorError> for ExplorerError {
    fn from(error: ExecutorError) -> Self {
        match error {
            ExecutorError::I2cError(kind) => ExplorerError::ExecutionFailed(kind),
            ExecutorError::ExecFailed => ExplorerError::ExecutionFailed(ErrorKind::Unknown),
            ExecutorError::BufferOverflow => ExplorerError::BufferOverflow,
            ExecutorError::BitFlags(e) => ExplorerError::BitFlags(e),
            ExecutorError::Explorer(e) => e,
        }
    }
}

/// Converts an `ErrorKind` into an `ExplorerError`.
impl From<ErrorKind> for ExplorerError {
    fn from(error: ErrorKind) -> Self {
        ExplorerError::DeviceNotFound(error)
    }
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutorError::I2cError(kind) => write!(f, "I2cError: {kind}"),
            ExecutorError::ExecFailed => f.write_str("ExecFailed"),
            ExecutorError::BufferOverflow => f.write_str("BufferOverflow"),
            ExecutorError::BitFlags(e) => write!(f, "BitFlagsError: {e}"),
            ExecutorError::Explorer(e) => write!(f, "ExplorerError: {e}"),
        }
    }
}
