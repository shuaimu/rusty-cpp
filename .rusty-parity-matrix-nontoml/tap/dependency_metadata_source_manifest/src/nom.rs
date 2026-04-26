//! Tap operations for nom's `IResult`, requires the feature `nom3`.
extern crate nom;

use self::nom::Err;
use self::nom::IResult;
use self::nom::Needed;

/// Tap operations for nom's `IResult`, requires the feature `nom3`.
pub trait TapNomOps<I, O, E> {
    /// Executes a closure if the value is `IResult::Done`.
    ///
    /// The closure will receive a tuple of (unparsed input, parsed output).
    fn tap_done<R, F: FnOnce((&mut I, &mut O)) -> R>(self, f: F) -> Self;

    /// Executes a closure if the value is `IResult::Error`.
    fn tap_error<R, F: FnOnce(&mut Err<E>) -> R>(self, f: F) -> Self;

    /// Executes a closure if the value is `IResult::Incomplete`.
    fn tap_incomplete<R, F: FnOnce(&mut Needed) -> R>(self, f: F) -> Self;
}

impl<I, O, E> TapNomOps<I, O, E> for IResult<I, O, E> {
    fn tap_done<R, F: FnOnce((&mut I, &mut O)) -> R>(mut self, f: F) -> Self {
        use self::nom::IResult::Done;
        if let &mut Done(ref mut rem, ref mut val) = &mut self {
            let _ = f((rem, val));
        }
        self
    }
    fn tap_error<R, F: FnOnce(&mut Err<E>) -> R>(mut self, f: F) -> Self {
        use self::nom::IResult::Error;
        if let &mut Error(ref mut err) = &mut self {
            let _ = f(&mut *err);
        }
        self
    }
    fn tap_incomplete<R, F: FnOnce(&mut Needed) -> R>(mut self, f: F) -> Self {
        use self::nom::IResult::Incomplete;
        if let &mut Incomplete(ref mut needed) = &mut self {
            let _ = f(needed);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::nom::ErrorKind;
    use super::*;
    type TestResult = IResult<&'static str, i32, u32>;

    #[test]
    fn done() {
        let d: TestResult = IResult::Done(" 24", 42);
        let mut n = 0;
        d.tap_done(|(rem, val)| {
            assert_eq!(val, &mut 42);
            if let Ok(p) = rem.trim().parse::<i32>() {
                n = p;
            }
        });
        assert_eq!(n, 24);
    }

    #[test]
    fn error() {
        let e: TestResult = IResult::Error(ErrorKind::Custom('t' as u32));
        let mut err_code = 0;
        e.tap_error(|e| {
            if let ErrorKind::Custom(c) = *e {
                err_code = c;
            }
        });
        assert_eq!(err_code, 116);
    }

    #[test]
    #[should_panic]
    fn incomplete() {
        let i: TestResult = IResult::Incomplete(Needed::Unknown);
        let mut more = 0;
        i.tap_incomplete(|i| {
            if let Needed::Size(s) = *i {
                more = s;
            } else {
                panic!();
            }
        });
    }
}
