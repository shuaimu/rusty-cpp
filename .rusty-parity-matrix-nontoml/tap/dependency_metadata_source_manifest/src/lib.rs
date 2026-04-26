//! A simple crate exposing tapping functionality for all types, and extended functionality for `Option`, `Result` & `Future`. 
//! 
//! The tap operation takes, and then returns, full ownership of the variable being tapped.
//! This means that the closure may have mutable access to the variable, even if the variable is otherwise immutable.
//! 
//! # Examples
//! 
//! Logging error values:
//! 
//! ```rust
//! # use tap::*;
//! let values: [Result<i32, &str>; 4] = [Ok(3), Err("foo"), Err("bar"), Ok(8)];
//!
//! let _ = values.iter().filter_map(|result|
//!     // print error information before discarding them
//!     result.tap_err(|error| eprintln!("Invalid entry: {}", error)).ok()
//! );
//! ```
//! 
//! Chaining methods:
//! 
//! ```rust
//! # use tap::*;
//! fn get_numbers() -> Vec<u32> {
//!    vec![4, 9, 1, 17, 3]
//! }
//! 
//! let mut old = get_numbers();
//! old.sort();
//! 
//! // can now be written like this instead
//! let new = get_numbers().tap(|data| data.sort());
//! 
//! assert_eq!(old, new)
//! ```
//! 
//! Reducing the amount of mutable variables:
//! 
//! ```rust
//! # use tap::*;
//! let tapped = [1, 2, 3]; // does not need to be mutable, preventing accidental mutations
//! let tapped = tapped.tap(|arr| {
//!     for elt in arr.iter_mut() {
//!         *elt *= 2;
//!     }
//! });
//! 
//! // instead of
//! let mut untapped = [1, 2, 3];
//! for elt in untapped.iter_mut() {
//!     *elt *= 2;
//! }
//! assert_eq!(tapped, untapped);
//! ```

#[cfg(feature = "future")]
pub use self::future::TapFutureOps;

#[cfg(feature = "future")]
mod future;

#[cfg(feature = "nom3")]
pub use self::nom::TapNomOps;

#[cfg(feature = "nom3")]
mod nom;

/// Tap operations for `bool`.
pub trait TapBooleanOps {
    /// Executes a closure if `self` is `true`.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// # use tap::*;
    /// let mut foo = 0;
    /// let boolean = false;
    /// assert_eq!(boolean.tap_true(|| foo += 5), false);
    /// assert_eq!(foo, 0);
    /// ```
    /// 
    /// ```rust
    /// # use tap::*;
    /// let mut foo = 0;
    /// let boolean = true;
    /// assert_eq!(boolean.tap_true(|| foo += 5), true);
    /// assert_eq!(foo, 5);
    /// ```
    fn tap_true<R, F: FnOnce() -> R>(self, f: F) -> Self;

    /// Executes a closure if `self` is `false`.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// # use tap::*;
    /// let mut foo = 0;
    /// let boolean = false;
    /// assert_eq!(boolean.tap_false(|| foo += 5), false);
    /// assert_eq!(foo, 5);
    /// ```
    /// 
    /// ```rust
    /// # use tap::*;
    /// let mut foo = 0;
    /// let boolean = true;
    /// assert_eq!(boolean.tap_false(|| foo += 5), true);
    /// assert_eq!(foo, 0);
    /// ```
    fn tap_false<R, F: FnOnce() -> R>(self, f: F) -> Self;
}

impl TapBooleanOps for bool {
    fn tap_true<R, F: FnOnce() -> R>(self, f: F) -> Self {
        if self {
            let _ = f();
        }
        self
    }

    fn tap_false<R, F: FnOnce() -> R>(self, f: F) -> Self {
        if !self {
            let _ = f();
        }
        self
    }
}

/// Tap operations for `Result`.
pub trait TapResultOps<T, E> {
    /// Executes a closure if the value is `Result::Ok(T)`.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// # use tap::*;
    /// let mut foo = 0;
    /// let res: Result<u32, u32> = Ok(4);
    /// assert_eq!(res.tap_ok(|&mut v| foo += v), Ok(4));
    /// assert_eq!(foo, 4);
    /// ```
    /// 
    /// ```rust
    /// # use tap::*;
    /// let mut foo = 0;
    /// let res: Result<u32, u32> = Err(4);
    /// assert_eq!(res.tap_ok(|&mut v| foo += v), Err(4));
    /// assert_eq!(foo, 0);
    /// ```
    fn tap_ok<R, F: FnOnce(&mut T) -> R>(self, f: F) -> Self;

    /// Executes a closure if the value is `Result::Err(E)`.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// # use tap::*;
    /// let mut foo = 0;
    /// let res: Result<u32, u32> = Ok(4);
    /// assert_eq!(res.tap_err(|&mut v| foo += v), Ok(4));
    /// assert_eq!(foo, 0);
    /// ```
    /// 
    /// ```rust
    /// # use tap::*;
    /// let mut foo = 0;
    /// let res: Result<u32, u32> = Err(4);
    /// assert_eq!(res.tap_err(|&mut v| foo += v), Err(4));
    /// assert_eq!(foo, 4);
    /// ```
    fn tap_err<R, F: FnOnce(&mut E) -> R>(self, f: F) -> Self;
}

impl<T, E> TapResultOps<T, E> for Result<T, E> {
    fn tap_ok<R, F: FnOnce(&mut T) -> R>(mut self, f: F) -> Self {
        if let Ok(mut val) = self.as_mut() {
            let _ = f(&mut val);
        }
        self
    }

    fn tap_err<R, F: FnOnce(&mut E) -> R>(mut self, f: F) -> Self {
        if let Err(mut val) = self.as_mut() {
            let _ = f(&mut val);
        }
        self
    }
}

/// Tap operations for `Option`.
pub trait TapOptionOps<T> {
    /// Executes a closure if the value is `Option::Some(T)`.
    ///  
    /// # Examples
    /// 
    /// ```rust
    /// # use tap::*;
    /// let mut foo = 0;
    /// let res: Option<u32> = Some(4);
    /// assert_eq!(res.tap_some(|&mut v| foo += v), Some(4));
    /// assert_eq!(foo, 4);
    /// ```
    /// 
    /// ```rust
    /// # use tap::*;
    /// let mut foo = 0;
    /// let res: Option<u32> = None;
    /// assert_eq!(res.tap_some(|&mut v| foo += v), None);
    /// assert_eq!(foo, 0);
    /// ```
    fn tap_some<R, F: FnOnce(&mut T) -> R>(self, f: F) -> Self;

    /// Executes a closure if the value is `Option::None`.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// # use tap::*;
    /// let mut foo = 0;
    /// let res: Option<u32> = Some(4);
    /// assert_eq!(res.tap_none(|| foo += 5), Some(4));
    /// assert_eq!(foo, 0);
    /// ```
    /// 
    /// ```rust
    /// # use tap::*;
    /// let mut foo = 0;
    /// let res: Option<u32> = None;
    /// assert_eq!(res.tap_none(|| foo += 5), None);
    /// assert_eq!(foo, 5);
    /// ```
    fn tap_none<R, F: FnOnce() -> R>(self, f: F) -> Self;
}

impl<T> TapOptionOps<T> for Option<T> {
    fn tap_some<R, F: FnOnce(&mut T) -> R>(mut self, f: F) -> Self {
        if let Some(val) = self.as_mut() {
            let _ = f(val);
        }
        self
    }

    fn tap_none<R, F: FnOnce() -> R>(self, f: F) -> Self {
        if self.is_none() {
            let _ = f();
        }
        self
    }
}

/// Tap operations for all types.
pub trait TapOps: Sized {
    /// Executes a closure on an object, discarding the result.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// # use tap::*;
    /// let mut max = 0;
    /// let data: [u32; 5] = [2, 8, 3, 4, 0];
    /// assert_eq!(
    ///     data.tap(|x| x.sort()).tap(|x| max += x.last().unwrap()), 
    ///     [0, 2, 3, 4, 8]
    /// );
    /// assert_eq!(max, 8);
    /// ```
    fn tap<R, F>(self, f: F) -> Self
        where F: FnOnce(&mut Self) -> R;
}

impl<T> TapOps for T where T: Sized {
    fn tap<R, F>(mut self, f: F) -> Self
        where F: FnOnce(&mut Self) -> R
    {
        let _ = f(&mut self);
        self
    }
}