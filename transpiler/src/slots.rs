//! Slot detection for the hand-override mechanism.
//!
//! When the transpiler emits a function/method body that contains a
//! marker indicating an incomplete or unsupported construct (e.g.
//! `// TODO: unhandled match pattern`, `// Rust-only ... skipped`), we
//! call that emission a "slot" — a candidate for hand-override in
//! large ports (the rustc-stdlib BTreeMap port is the motivating case).
//!
//! This module is a post-hoc scanner. After the codegen pipeline has
//! produced a `.cppm` file, we walk it looking for known marker shapes
//! and record each occurrence with its source location and (best-effort)
//! enclosing C++ symbol. The aggregated list is written to a manifest
//! file at the end of crate-mode transpilation so the human reviewer
//! has a single place to see which generated symbols need attention.
//!
//! Why a post-hoc scan and not direct instrumentation of every emit
//! site: the transpiler has dozens of points where it emits TODO-style
//! markers, scattered across the codegen state machine. Threading slot
//! recording through every one of them would be invasive; the scanner
//! covers them all with a single regex-ish pass. Cost is ~O(file size)
//! after writing — negligible compared to transpilation itself.
//!
//! The marker contract is intentionally loose: any line that looks
//! like a TODO from the transpiler counts. New TODO shapes added to
//! codegen.rs get picked up automatically as long as they match one
//! of the documented prefixes below.

use std::collections::BTreeMap;

/// One slot occurrence: the location of a single TODO/skipped marker
/// in a generated `.cppm` file.
#[derive(Debug, Clone)]
pub struct Slot {
    /// Path to the generated `.cppm` file (as the user sees it, e.g.
    /// `btree_port.btree.node.cppm` — relative to the output dir).
    pub file: String,
    /// 1-based line number of the marker line.
    pub line: usize,
    /// Marker kind tag (one of the variants from `MarkerKind`).
    pub marker_kind: MarkerKind,
    /// Verbatim marker line, trimmed of leading/trailing whitespace.
    pub marker_text: String,
    /// Best-effort enclosing C++ symbol (function or method name). May
    /// be `None` when the scanner can't recover the surrounding scope
    /// (e.g. the marker is at file scope, or inside a deeply nested
    /// template that the heuristic can't unwind).
    pub enclosing_symbol: Option<String>,
}

/// Categories of marker we recognize. Keeping them as a closed enum
/// (rather than a free-form string) makes it easy to tell at a glance
/// whether the manifest is dominated by, say, the `Rust-only ...
/// skipped` family vs the `TODO transpiler` family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    /// `// TODO transpiler: …` or `/* TODO transpiler: … */`. Inserted
    /// when codegen explicitly couldn't handle a construct and is
    /// asking the user to patch the arm manually.
    TodoTranspiler,
    /// `// TODO(interface_traits): …`, `// TODO(<other_subsystem>): …`.
    /// A subsystem-specific TODO.
    TodoTagged,
    /// Plain `// TODO: …`. Generic catch-all from various lowering
    /// paths.
    Todo,
    /// `// Rust-only ... skipped …`. The transpiler dropped a Rust
    /// construct that has no direct C++ equivalent (e.g.
    /// `use Enum::*;` imports, nested `impl Drop` blocks in function
    /// bodies, `unsafe impl Sync/Send` markers).
    SkippedRustOnly,
}

impl MarkerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MarkerKind::TodoTranspiler => "todo_transpiler",
            MarkerKind::TodoTagged => "todo_tagged",
            MarkerKind::Todo => "todo",
            MarkerKind::SkippedRustOnly => "skipped_rust_only",
        }
    }
}

/// Classify a single trimmed line. Returns `None` when the line is
/// not a marker we care about.
fn classify_marker_line(trimmed: &str) -> Option<MarkerKind> {
    if trimmed.starts_with("// TODO transpiler") || trimmed.starts_with("/* TODO transpiler") {
        return Some(MarkerKind::TodoTranspiler);
    }
    // The `TODO(` form is used by subsystems like `// TODO(interface_traits): …`.
    if trimmed.starts_with("// TODO(") {
        return Some(MarkerKind::TodoTagged);
    }
    // Generic catch-all. Must come after the more-specific forms.
    // Accept both `// TODO ` (with description after a space) and
    // `// TODO:` (with colon), as both appear in the codebase.
    if trimmed.starts_with("// TODO:") || trimmed.starts_with("// TODO ") {
        return Some(MarkerKind::Todo);
    }
    if (trimmed.starts_with("// Rust-only") || trimmed.starts_with("// #[cfg(test)]"))
        && (trimmed.contains("skipped") || trimmed.contains("omitted"))
    {
        return Some(MarkerKind::SkippedRustOnly);
    }
    // Inline `/* TODO transpiler: … */` markers can appear mid-line
    // (the transpiler embeds them in match-arm IIFE conditions when
    // it can't resolve a bare-glob variant). Catch those too — but
    // only when the comment opens at column position 0 of the trim,
    // i.e. the line itself starts with a marker. Mid-line markers
    // are reported by the secondary `contains` check in `detect_slots`.
    if trimmed.starts_with("/* TODO transpiler") {
        return Some(MarkerKind::TodoTranspiler);
    }
    None
}

/// Walk one generated C++ file and return every slot it contains.
///
/// `file` is the relative path label that ends up in the manifest;
/// `content` is the full text of the generated `.cppm`.
pub fn detect_slots(file: &str, content: &str) -> Vec<Slot> {
    let lines: Vec<&str> = content.lines().collect();
    let mut out = Vec::new();
    for (idx, raw_line) in lines.iter().enumerate() {
        let trimmed = raw_line.trim();
        // Whole-line marker (starts with the marker prefix).
        if let Some(kind) = classify_marker_line(trimmed) {
            let enclosing = find_enclosing_symbol(&lines, idx);
            out.push(Slot {
                file: file.to_string(),
                line: idx + 1,
                marker_kind: kind,
                marker_text: trimmed.to_string(),
                enclosing_symbol: enclosing,
            });
            continue;
        }
        // Mid-line `/* TODO transpiler */` marker inside an emitted
        // expression. Only count these when they're not already
        // covered by a whole-line match above.
        if raw_line.contains("/* TODO transpiler") {
            let enclosing = find_enclosing_symbol(&lines, idx);
            out.push(Slot {
                file: file.to_string(),
                line: idx + 1,
                marker_kind: MarkerKind::TodoTranspiler,
                marker_text: trimmed.to_string(),
                enclosing_symbol: enclosing,
            });
        }
    }
    out
}

/// Heuristic: walk backwards from `marker_line` looking for the line
/// that opened the enclosing C++ scope. Track brace depth so we don't
/// confuse the enclosing function with a sibling lambda body. Cap the
/// search at 200 lines back to keep the cost bounded on huge files.
///
/// Returns the function/method "name(args)" portion of the line, or
/// `None` if no plausible candidate is found.
fn find_enclosing_symbol(lines: &[&str], marker_line: usize) -> Option<String> {
    let mut depth: i32 = 0;
    let start = marker_line;
    let end = marker_line.saturating_sub(200);
    for back in (end..start).rev() {
        let line = lines[back];
        // Count braces on this line. Order matters here (close-then-open
        // mirrors a single-line `} else if (...) {` shape).
        let closes = line.chars().filter(|&c| c == '}').count() as i32;
        let opens = line.chars().filter(|&c| c == '{').count() as i32;
        depth += closes;
        depth -= opens;
        if depth < 0 {
            // We've found an opening brace that has no matching close
            // between us and the marker — that's the enclosing scope.
            if let Some(sig) = extract_signature_from_opener(lines, back) {
                return Some(sig);
            }
            // Reset depth and keep searching upward in case the brace
            // we found was a struct/namespace opener rather than a
            // function (the extractor returned None).
            depth = 0;
        }
    }
    None
}

/// Given a candidate "opener" line (one that opened a `{` scope), try
/// to recover a function/method signature from it. We're permissive:
/// any non-keyword identifier followed by `(` is treated as a function
/// name. Returns the bare `name` token (without arg list) for brevity
/// in the manifest.
fn extract_signature_from_opener(lines: &[&str], opener_line: usize) -> Option<String> {
    // The signature may span multiple lines (template params, return
    // type with nested templates, etc.). Gather a small window.
    let window_start = opener_line.saturating_sub(4);
    let joined: String = lines[window_start..=opener_line].join(" ");
    let trimmed = joined.trim();
    // The opener line must end with '{' for this to be a body opener.
    // (Note: we use `lines[opener_line]` here, not the joined window.)
    let last_line_trimmed = lines[opener_line].trim_end();
    if !last_line_trimmed.ends_with('{') {
        return None;
    }
    // Walk backwards from the last `{` to find the matching `(`-`)`
    // pair, then back up to the function name.
    let close_paren = trimmed.rfind(')')?;
    // Find the matching `(`.
    let mut depth = 1i32;
    let bytes = trimmed.as_bytes();
    let mut open_paren: Option<usize> = None;
    for i in (0..close_paren).rev() {
        match bytes[i] {
            b')' => depth += 1,
            b'(' => {
                depth -= 1;
                if depth == 0 {
                    open_paren = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let open_paren = open_paren?;
    let before_paren = &trimmed[..open_paren];
    // The function name is the last word before the open paren.
    let name_end = before_paren.trim_end();
    let name_start = name_end
        .rfind(|c: char| c.is_whitespace() || c == '&' || c == '*' || c == ':')
        .map(|i| i + 1)
        .unwrap_or(0);
    let name = name_end[name_start..].trim();
    if name.is_empty() {
        return None;
    }
    // Filter out keywords / control-flow that look like function calls.
    if matches!(
        name,
        "if" | "for" | "while" | "switch" | "catch" | "return" | "else"
    ) {
        return None;
    }
    Some(name.to_string())
}

/// Format the manifest as a human-readable Markdown file. Slots are
/// grouped by file, then sorted by line number within each file.
pub fn format_manifest(slots: &[Slot]) -> String {
    let mut out = String::new();
    out.push_str("# Rusty-CPP transpiler — hand-override slots\n\n");
    out.push_str(&format!(
        "{} slot(s) requiring hand-attention across {} file(s).\n\n",
        slots.len(),
        count_distinct_files(slots)
    ));
    if slots.is_empty() {
        out.push_str("_No slots detected. The transpiler did not emit any \
                      TODO/skipped markers in this run._\n");
        return out;
    }
    out.push_str("## Marker kind summary\n\n");
    for kind in [
        MarkerKind::TodoTranspiler,
        MarkerKind::TodoTagged,
        MarkerKind::Todo,
        MarkerKind::SkippedRustOnly,
    ] {
        let count = slots.iter().filter(|s| s.marker_kind == kind).count();
        if count > 0 {
            out.push_str(&format!("- `{}`: {}\n", kind.as_str(), count));
        }
    }
    out.push_str("\n## Per-file detail\n\n");
    // Group by file with stable ordering.
    let mut by_file: BTreeMap<&str, Vec<&Slot>> = BTreeMap::new();
    for s in slots {
        by_file.entry(s.file.as_str()).or_default().push(s);
    }
    for (file, file_slots) in by_file {
        out.push_str(&format!("### `{}`\n\n", file));
        for slot in file_slots {
            let sym = slot
                .enclosing_symbol
                .as_deref()
                .unwrap_or("<file scope>");
            out.push_str(&format!(
                "- **L{}** `{}` ({}) — `{}`\n",
                slot.line,
                sym,
                slot.marker_kind.as_str(),
                truncate_for_manifest(&slot.marker_text, 120),
            ));
        }
        out.push('\n');
    }
    out
}

fn count_distinct_files(slots: &[Slot]) -> usize {
    let mut set = std::collections::BTreeSet::new();
    for s in slots {
        set.insert(s.file.as_str());
    }
    set.len()
}

fn truncate_for_manifest(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        let mut out: String = text.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_marker_line_recognizes_each_kind() {
        assert_eq!(
            classify_marker_line("// TODO transpiler: unresolved variant"),
            Some(MarkerKind::TodoTranspiler)
        );
        assert_eq!(
            classify_marker_line("// TODO(interface_traits): not yet supported"),
            Some(MarkerKind::TodoTagged)
        );
        assert_eq!(
            classify_marker_line("// TODO: unhandled match pattern"),
            Some(MarkerKind::Todo)
        );
        assert_eq!(
            classify_marker_line(
                "// Rust-only namespace import skipped for type path: using namespace Foo;"
            ),
            Some(MarkerKind::SkippedRustOnly)
        );
        // Non-marker lines.
        assert_eq!(classify_marker_line("int x = 0;"), None);
        assert_eq!(classify_marker_line("// regular comment"), None);
        // `// TODO` without a colon or space follower (`// TODOLATER`) is
        // not a marker.
        assert_eq!(classify_marker_line("// TODOLATER something"), None);
    }

    #[test]
    fn detect_slots_finds_each_marker_kind() {
        let content = r#"
void f() {
    // TODO: unhandled match pattern
    int x = 0;
}

void g() {
    // TODO(interface_traits): trait has associated constants
}

// TODO transpiler: a transpiler-internal marker
void h() {
    foo(/* TODO transpiler: bare-glob variant */ true);
}

// Rust-only namespace import skipped for: using namespace Bar;
void i() {}
"#;
        let slots = detect_slots("test.cppm", content);
        let kinds: Vec<MarkerKind> = slots.iter().map(|s| s.marker_kind).collect();
        assert!(kinds.contains(&MarkerKind::Todo));
        assert!(kinds.contains(&MarkerKind::TodoTagged));
        assert!(kinds.contains(&MarkerKind::TodoTranspiler));
        assert!(kinds.contains(&MarkerKind::SkippedRustOnly));
        // The mid-line `/* TODO transpiler */` is also picked up (so
        // we have at least 5 total: two transpiler markers + the
        // three others).
        let transpiler_count = slots
            .iter()
            .filter(|s| s.marker_kind == MarkerKind::TodoTranspiler)
            .count();
        assert_eq!(
            transpiler_count, 2,
            "expected both the standalone and mid-line transpiler markers, got slots: {:#?}",
            slots
        );
    }

    #[test]
    fn find_enclosing_symbol_recovers_function_name() {
        let content = r#"
template<typename T>
void my_function(int x) {
    if (x > 0) {
        // TODO: something
    }
}
"#;
        let slots = detect_slots("test.cppm", content);
        assert_eq!(slots.len(), 1);
        let sym = slots[0]
            .enclosing_symbol
            .as_deref()
            .expect("expected enclosing symbol");
        // The heuristic should recover `my_function` (the innermost
        // open scope is the `if`, but `if` is filtered out as a
        // keyword, so we walk further up).
        assert_eq!(sym, "my_function");
    }

    #[test]
    fn find_enclosing_symbol_handles_method_with_qualified_name() {
        let content = r#"
template<typename A>
SearchResult<A> SmallVec<A>::search_tree(const Q& key) {
    // TODO: unhandled
}
"#;
        let slots = detect_slots("test.cppm", content);
        assert_eq!(slots.len(), 1);
        let sym = slots[0].enclosing_symbol.as_deref().unwrap();
        // The heuristic returns the bare method name (the `Type::`
        // prefix is stripped because `:` is in the trim set).
        assert_eq!(sym, "search_tree");
    }

    #[test]
    fn format_manifest_groups_by_file_and_summarizes_kinds() {
        let slots = vec![
            Slot {
                file: "a.cppm".to_string(),
                line: 10,
                marker_kind: MarkerKind::Todo,
                marker_text: "// TODO: x".to_string(),
                enclosing_symbol: Some("foo".to_string()),
            },
            Slot {
                file: "a.cppm".to_string(),
                line: 20,
                marker_kind: MarkerKind::SkippedRustOnly,
                marker_text: "// Rust-only X skipped".to_string(),
                enclosing_symbol: None,
            },
            Slot {
                file: "b.cppm".to_string(),
                line: 5,
                marker_kind: MarkerKind::TodoTranspiler,
                marker_text: "// TODO transpiler: y".to_string(),
                enclosing_symbol: Some("bar".to_string()),
            },
        ];
        let manifest = format_manifest(&slots);
        assert!(manifest.contains("3 slot(s)"));
        assert!(manifest.contains("2 file(s)"));
        assert!(manifest.contains("`todo`: 1"));
        assert!(manifest.contains("`skipped_rust_only`: 1"));
        assert!(manifest.contains("`todo_transpiler`: 1"));
        assert!(manifest.contains("### `a.cppm`"));
        assert!(manifest.contains("### `b.cppm`"));
        // Per-slot lines mention the enclosing symbol when known and
        // `<file scope>` otherwise.
        assert!(manifest.contains("`foo`"));
        assert!(manifest.contains("`bar`"));
        assert!(manifest.contains("<file scope>"));
    }

    #[test]
    fn format_manifest_handles_empty_slot_list() {
        let manifest = format_manifest(&[]);
        assert!(manifest.contains("0 slot(s)"));
        assert!(manifest.contains("No slots detected"));
    }

    #[test]
    fn truncate_for_manifest_clips_long_lines() {
        let long = "a".repeat(200);
        let out = truncate_for_manifest(&long, 50);
        assert_eq!(out.chars().count(), 50);
        assert!(out.ends_with('…'));
    }
}
