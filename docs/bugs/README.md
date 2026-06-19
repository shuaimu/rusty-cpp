# Bug Reports

This directory contains bug reports for the rusty-cpp borrow checker.

## Active Bugs

| Bug | Summary | Status |
|-----|---------|--------|
| [Cell Const Propagation](bug_report_cell_const_propagation.md) | Const propagation incorrectly flags `Cell::set()` calls | Open |
| [Unannotated Headers](bug_report_checking_unannotated_headers.md) | Checker analyzes code from third-party headers without `@safe` annotations | Open |
| [Method Call False Positives](bug_report_this_borrow_false_positives.md) | False positives for method calls on `this` and return value tracking | Partial Fix |
| [`break` in `match` arm targets `switch`](bug_report_break_in_match_arm_targets_switch_not_loop.md) | Codegen: `break` in a match arm breaks the lowered `switch`, not the enclosing loop; transpiler emits dead `break; break;` (BTreeMap::merge infinite loop) | Open |

## Bug Report Template

When filing a new bug report, use this minimal template:

```markdown
# Bug: [Short Description]

## Summary
One-line description of the issue.

## Minimal Reproduction
\`\`\`cpp
// Smallest code that reproduces the bug
\`\`\`

## Expected
What should happen.

## Actual
What actually happens (include error message).

## Workaround
If any.
```
