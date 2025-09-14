// Macro for debug logging - only active in debug builds
#[macro_export]
#[cfg(debug_assertions)]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        eprintln!($($arg)*);
    };
}

#[macro_export]
#[cfg(not(debug_assertions))]
macro_rules! debug_println {
    ($($arg:tt)*) => {};
}