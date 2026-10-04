// @error driver-cfg-unevaluated
// @expect-error survives in a position this pass does not evaluate
// A driver cfg on an expression: not an attribute owner rustc's cfg stripping
// and this pass agree on, so it fails closed.
pub fn f() -> u64 {
    #[cfg(verus_keep_ghost)]
    1
}
