// Macro for debug logging - enabled via RUSTY_CPP_DEBUG env var
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        if std::env::var("RUSTY_CPP_DEBUG").is_ok() {
            eprintln!($($arg)*);
        }
    };
}