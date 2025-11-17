// Macro for debug logging - disabled in release builds
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        // Debug output disabled
    };
}