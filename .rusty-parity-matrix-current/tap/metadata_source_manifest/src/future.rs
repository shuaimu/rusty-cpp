//! Tap operations for `Future`, requires the feature `future`.
extern crate futures;

use self::futures::{Async, Future};

/// Tap operations for `Future`, requires the feature `future`.
pub trait TapFutureOps<T, E> {
    /// Executes a closure if the value is `Async::Ready(T)`.
    fn tap_ready<R, F: FnOnce(&T) -> R>(self, f: F) -> Self;

    // Executes a closure if the value is `Async::NotReady`.
    fn tap_not_ready<R, F: FnOnce() -> R>(self, f: F) -> Self;

    // Executes a closure if the value is `Err(E)`.
    fn tap_err<R, F: FnOnce(&E) -> R>(self, f: F) -> Self;
}

impl<T, E, FUT: Future<Item = T, Error = E>> TapFutureOps<T, E> for FUT {
    fn tap_ready<R, F: FnOnce(&T) -> R>(mut self, f: F) -> Self {
        if let Ok(Async::Ready(ref val)) = self.poll() {
            let _ = f(val);
        }
        self
    }

    fn tap_not_ready<R, F: FnOnce() -> R>(mut self, f: F) -> Self {
        if let Ok(Async::NotReady) = self.poll() {
            let _ = f();
        }
        self
    }

    fn tap_err<R, F: FnOnce(&E) -> R>(mut self, f: F) -> Self {
        if let Err(ref val) = self.poll() {
            let _ = f(val);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready() {
        let mut foo = 0;
        let future = futures::future::result::<i32, i32>(Ok(5));

        let _ = future.tap_ready(|x| foo += *x);
        assert_eq!(foo, 5);
    }

    #[test]
    fn not_ready() {
        let mut foo = 0;
        let future = futures::future::empty::<i32, i32>();

        match future.tap_not_ready(|| foo += 5).poll() {
            Ok(Async::NotReady) => assert_eq!(foo, 5),
            _ => unreachable!()
        }
    }

    #[test]
    fn error() {
        let mut foo = 0;
        let future = futures::future::result::<i32, i32>(Err(5));

        let _ = future.tap_err(|x| foo += *x);
        assert_eq!(foo, 5);
    }
}
