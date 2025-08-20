#[macro_export]
macro_rules! adapt_serial {
    ($name:ident, nb_write = $write_fn:ident $(, flush = $flush_fn:ident)?) => {
        pub struct $name<T>(pub T);

        impl<T> $name<T>
        where
            T: $crate::compat::serial_compat::SerialCompat,
        {
            /// Return a `CoreWriteAdapter` that implements `core::fmt::Write`.
            pub fn as_core_write(&mut self) -> $crate::compat::adapt::CoreWriteAdapter<&mut T> {
                $crate::compat::adapt::CoreWriteAdapter(&mut self.0)
            }
        }

        impl<T> core::fmt::Write for $name<T>
        where
            T: $crate::compat::serial_compat::SerialCompat,
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let mut adapter = $crate::compat::adapt::CoreWriteAdapter(&mut self.0);
                adapter.write_str(s)
            }
        }
    };
}
