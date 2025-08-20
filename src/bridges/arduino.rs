// src/bridges/arduino.rs

#![cfg(feature = "arduino")]

use arduino_hal::Usart;
use dvcdbg::prelude::*;

adapt_serial!(UnoSerial: Usart);
