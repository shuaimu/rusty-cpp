pub trait Backend {
    fn poll(&mut self) -> u32;
}

pub fn with_backend(mut backend: Box<dyn Backend>) -> u32 {
    backend.poll() * 2
}

// A test-only module whose file lives outside src/: not part of the crate.
#[cfg(test)]
#[path = "../tests/helpers.rs"]
mod helper_tests;
