#[macro_export]
macro_rules! pretend_assert {
    ($expression:expr) => {
        let _ = $expression;
    };
}
