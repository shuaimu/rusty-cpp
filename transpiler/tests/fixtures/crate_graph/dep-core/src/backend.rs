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

/// An OS-backend-shaped trait over a type alias: an implementor's inherent
/// `close(&mut self, fd: i32)` and its trait `close(&mut self, fd: RawFd)`
/// are one C++ signature.
pub type RawFd = i32;

pub trait FdBackend {
    fn close(&mut self, fd: RawFd) -> u32;
    fn label(&self) -> u32;
}

pub fn with_fd_backend(mut backend: Box<dyn FdBackend>) -> u32 {
    backend.close(4) + backend.label()
}
