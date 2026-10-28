//! either's shape (the module is not named `either`: that name is the
//! runtime-mapped crate): a conditional `impl Deref` whose `Target` is a type
//! parameter's projection, and the enum instantiated with a parameter that
//! does NOT satisfy the bound. The C++ class template merges the impl
//! unconditionally, so its member declarations must be well-formed for
//! `Either<u32, u32>` too: the alias and the explicit return spell the probe
//! (`rusty::detail::assoc_Target<L>`), never `typename L::Target`.
use std::ops::Deref;

pub struct Cell(pub u32);

impl Deref for Cell {
    type Target = u32;

    fn deref(&self) -> &u32 {
        &self.0
    }
}

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: Deref, R: Deref<Target = L::Target>> Deref for Either<L, R> {
    type Target = L::Target;

    fn deref(&self) -> &Self::Target {
        match self {
            Either::Left(l) => &**l,
            Either::Right(r) => &**r,
        }
    }
}

/// `Either<u32, u32>` only instantiates the class; `Either<Cell, Cell>`
/// derefs through it: 5 + 40 = 45.
pub fn either_total() -> u32 {
    let plain: Either<u32, u32> = Either::Right(5);
    let cell: Either<Cell, Cell> = Either::Left(Cell(40));
    let plain_value = match plain {
        Either::Left(v) => v,
        Either::Right(v) => v,
    };
    plain_value + *cell
}
