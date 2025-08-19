/// Error wrapper for adapter
#[derive(Debug)]
pub enum AdaptError<E> {
    Other(E),
}

#[macro_export]
macro_rules! adapt_serial {
    ($name:ident, nb_write = $write_fn:ident $(, flush = $flush_fn:ident)?) => {
        pub struct $name<T>(pub T);

        // ========================
        // embedded-hal 1.0 support
        // ========================
        #[cfg(feature = "ehal_1_0")]
        impl<T> embedded_io::Write for $name<T>
        where
            T: $crate::compat::serial_compat::SerialCompat,
        {
            type Error = $crate::compat::serial::AdaptError<T::Error>;

            fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
                for &b in buf {
                    self.0.write(b).map_err($crate::compat::serial::AdaptError::Other)?;
                }
                Ok(buf.len())
            }

            fn flush(&mut self) -> Result<(), Self::Error> {
                self.0.flush().map_err($crate::compat::serial::AdaptError::Other)
            }
        }

        #[cfg(feature = "ehal_1_0")]
        impl<T> core::fmt::Write for $name<T>
        where
            T: $crate::compat::serial_compat::SerialCompat,
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                <Self as embedded_io::Write>::write_all(self, s.as_bytes())
                    .map_err(|_| core::fmt::Error)
            }
        }

        // ========================
        // embedded-hal 0.2.x support
        // ========================
        #[cfg(feature = "ehal_0_2")]
        impl<T> embedded_io::Write for $name<T>
        where
            T: $crate::compat::serial_compat::SerialCompat,
        {
            type Error = $crate::compat::serial::AdaptError<T::Error>;

            fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
                for &b in buf {
                    self.0.write(b).map_err($crate::compat::serial::AdaptError::Other)?;
                }
                Ok(buf.len())
            }

            fn flush(&mut self) -> Result<(), Self::Error> {
                self.0.flush().map_err($crate::compat::serial::AdaptError::Other)
            }
        }

        #[cfg(feature = "ehal_0_2")]
        impl<T> core::fmt::Write for $name<T>
        where
            T: $crate::compat::serial_compat::SerialCompat,
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                <Self as embedded_io::Write>::write_all(self, s.as_bytes())
                    .map_err(|_| core::fmt::Error)
            }
        }
    };
}
