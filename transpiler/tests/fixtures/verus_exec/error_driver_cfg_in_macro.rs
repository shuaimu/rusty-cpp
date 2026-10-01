// @error driver-cfg-in-macro
// @expect-error `thread_local!` mentions a Verus driver cfg inside its tokens
use std::cell::Cell;

thread_local! {
    #[cfg(verus_keep_ghost)]
    static GHOST: Cell<u64> = Cell::new(0);
}
