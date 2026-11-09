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

/// Lion's `OsInterrupt` shape: a trait object shared through `Arc`, coerced
/// from an implementor's own `Arc<T>`. The dependency's implementor is
/// reached through its trait lane, a consumer's through its members.
pub trait Signal {
    fn signal(&self) -> u32;
}

pub struct Bell {
    pub tone: u32,
}

impl Signal for Bell {
    fn signal(&self) -> u32 {
        self.tone
    }
}

pub fn signal_twice(signal: std::sync::Arc<dyn Signal>) -> u32 {
    let again = signal.clone();
    signal.signal() + again.signal()
}

/// (40 + 40) for the dependency's own `Bell`.
pub fn bell_total() -> u32 {
    let bell = std::sync::Arc::new(Bell { tone: 40 });
    signal_twice(bell)
}
