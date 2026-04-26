extern crate tap;

use tap::*;

#[test]
fn filter_map() {
    let values: &[Result<i32, &str>] = &[Ok(3), Err("foo"), Err("bar"), Ok(8)];
    let _ = values.iter().filter_map(|result| {
        // It is especially useful in filter maps, allowing error information to
        // be logged/printed before the information is discarded.
        result
            .tap_err(|error| println!("Invalid entry: {}", error))
            .ok()
    });
}

#[test]
fn basic() {
    let mut foo = 5;

    // The `tap` extension can be used on all types
    if 10.tap(|v| foo += *v) > 0 {
        assert_eq!(foo, 15);
    }

    // Results have `tap_err` & `tap_ok` available.
    let _: Result<i32, i32> = Err(5).tap_err(|e| foo = *e);
    assert_eq!(foo, 5);

    // Options have `tap_some` & `tap_none` available.
    let _: Option<i32> = None.tap_none(|| foo = 10);
    assert_eq!(foo, 10);
}
