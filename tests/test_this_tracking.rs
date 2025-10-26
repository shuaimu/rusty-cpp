use rusty_cpp::analysis::this_tracking::ThisPointerTracker;
use rusty_cpp::parser::MethodQualifier;
use rusty_cpp::ir::BorrowKind;

#[test]
fn test_const_method_restrictions() {
    let tracker = ThisPointerTracker::new(Some(MethodQualifier::Const));

    // Can read
    assert!(tracker.can_read_member("field").is_ok());

    // Cannot modify
    assert!(tracker.can_modify_member("field").is_err());

    // Cannot move
    assert!(tracker.can_move_member("field").is_err());

    // Can borrow immutably
    assert!(tracker.can_borrow_member("field", BorrowKind::Immutable).is_ok());

    // Cannot borrow mutably
    assert!(tracker.can_borrow_member("field", BorrowKind::Mutable).is_err());
}

#[test]
fn test_nonconst_method_restrictions() {
    let tracker = ThisPointerTracker::new(Some(MethodQualifier::NonConst));

    // Can read
    assert!(tracker.can_read_member("field").is_ok());

    // Can modify
    assert!(tracker.can_modify_member("field").is_ok());

    // CANNOT move (key restriction!)
    assert!(tracker.can_move_member("field").is_err());
    assert!(tracker.can_move_member("field").unwrap_err().contains("&mut self"));

    // Can borrow mutably
    assert!(tracker.can_borrow_member("field", BorrowKind::Mutable).is_ok());
}

#[test]
fn test_rvalue_method_permissions() {
    let tracker = ThisPointerTracker::new(Some(MethodQualifier::RvalueRef));

    // Can read
    assert!(tracker.can_read_member("field").is_ok());

    // Can modify
    assert!(tracker.can_modify_member("field").is_ok());

    // CAN move (full ownership!)
    assert!(tracker.can_move_member("field").is_ok());

    // Can borrow mutably
    assert!(tracker.can_borrow_member("field", BorrowKind::Mutable).is_ok());
}

#[test]
fn test_move_tracking() {
    let mut tracker = ThisPointerTracker::new(Some(MethodQualifier::RvalueRef));

    // Can move initially
    assert!(tracker.can_move_member("field").is_ok());

    // Mark as moved
    tracker.mark_field_moved("field".to_string());

    // Cannot read after move
    assert!(tracker.can_read_member("field").is_err());

    // Cannot move again
    assert!(tracker.can_move_member("field").is_err());

    // Cannot borrow after move
    assert!(tracker.can_borrow_member("field", BorrowKind::Immutable).is_err());
}

#[test]
fn test_borrow_conflicts() {
    let mut tracker = ThisPointerTracker::new(Some(MethodQualifier::NonConst));

    // Create mutable borrow
    tracker.mark_field_borrowed("field".to_string(), BorrowKind::Mutable);

    // Cannot create another borrow while mutably borrowed
    assert!(tracker.can_borrow_member("field", BorrowKind::Immutable).is_err());
    assert!(tracker.can_borrow_member("field", BorrowKind::Mutable).is_err());

    // Clear the borrow
    tracker.clear_field_borrow("field");

    // Now can borrow again
    assert!(tracker.can_borrow_member("field", BorrowKind::Immutable).is_ok());
}

#[test]
fn test_multiple_immutable_borrows() {
    let mut tracker = ThisPointerTracker::new(Some(MethodQualifier::NonConst));

    // Create immutable borrow
    tracker.mark_field_borrowed("field".to_string(), BorrowKind::Immutable);

    // Can create another immutable borrow
    assert!(tracker.can_borrow_member("field", BorrowKind::Immutable).is_ok());

    // Cannot create mutable borrow
    assert!(tracker.can_borrow_member("field", BorrowKind::Mutable).is_err());
}
