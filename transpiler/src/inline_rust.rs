use crate::transpile;
use crate::types::UserTypeMap;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const IF_RUSTYCPP_RUST: &str = "#if RUSTYCPP_RUST";
const ELSE_DIRECTIVE: &str = "#else";
const ENDIF_DIRECTIVE: &str = "#endif";
const RUST_BEGIN_PREFIX: &str = "/*RUSTYCPP:RUST-BEGIN";
const RUST_END_PREFIX: &str = "/*RUSTYCPP:RUST-END";
const LEGACY_AT_RUST_PREFIX: &str = "@rust";
const GEN_BEGIN_PREFIX: &str = "/*RUSTYCPP:GEN-BEGIN ";
const GEN_END_PREFIX: &str = "/*RUSTYCPP:GEN-END id=";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineRustMode {
    Check,
    Rewrite,
}

#[derive(Debug, Clone)]
pub struct InlineRustOptions {
    pub mode: InlineRustMode,
    pub files: Vec<PathBuf>,
}

pub fn run_inline_rust(options: &InlineRustOptions) -> Result<(), String> {
    if options.files.is_empty() {
        return Err("inline-rust: at least one path is required".to_string());
    }

    for path in &options.files {
        process_file(path, options.mode)?;
    }
    Ok(())
}

fn process_file(path: &Path, mode: InlineRustMode) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("{}: failed to read file: {}", path.display(), e))?;
    let blocks = parse_blocks(path, &content)?;
    if blocks.is_empty() {
        println!("inline-rust skip: {} (no block markers)", path.display());
        return Ok(());
    }

    match mode {
        InlineRustMode::Check => {
            for block in &blocks {
                let generated = block.generated_region.as_ref().ok_or_else(|| {
                    format!(
                        "{}:{}: missing generated region for block id={} (run --rewrite)",
                        path.display(),
                        block.if_line,
                        block.id
                    )
                })?;
                if generated.version != "1" {
                    return Err(format!(
                        "{}:{}: unsupported GEN marker version {}; expected 1",
                        path.display(),
                        block.if_line,
                        generated.version
                    ));
                }
                if generated.rust_sha256 != block.rust_hash {
                    return Err(format!(
                        "{}:{}: hash mismatch for id={} (marker={}, expected={})",
                        path.display(),
                        block.if_line,
                        block.id,
                        generated.rust_sha256,
                        block.rust_hash
                    ));
                }
            }
            println!(
                "inline-rust check: {} ({} block(s))",
                path.display(),
                blocks.len()
            );
            Ok(())
        }
        InlineRustMode::Rewrite => {
            let rewritten = rewrite_content(path, &content, &blocks)?;
            if rewritten != content {
                fs::write(path, rewritten)
                    .map_err(|e| format!("{}: failed to write file: {}", path.display(), e))?;
            }
            println!(
                "inline-rust rewrite: {} ({} block(s))",
                path.display(),
                blocks.len()
            );
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LineSpan {
    start: usize,
    end: usize,
}

fn collect_line_spans(content: &str) -> Vec<LineSpan> {
    let mut spans = Vec::new();
    let mut line_start = 0usize;
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            spans.push(LineSpan {
                start: line_start,
                end: idx + 1,
            });
            line_start = idx + 1;
        }
    }
    if line_start < content.len() {
        spans.push(LineSpan {
            start: line_start,
            end: content.len(),
        });
    }
    spans
}

fn line_trimmed<'a>(content: &'a str, line: &LineSpan) -> &'a str {
    content[line.start..line.end]
        .trim_end_matches('\n')
        .trim_end_matches('\r')
        .trim()
}

fn line_indent(content: &str, line: &LineSpan) -> String {
    content[line.start..line.end]
        .chars()
        .take_while(|c| c.is_ascii_whitespace() && *c != '\n' && *c != '\r')
        .collect()
}

fn next_nonempty_line(lines: &[LineSpan], content: &str, mut idx: usize) -> Option<usize> {
    while idx < lines.len() {
        if !line_trimmed(content, &lines[idx]).is_empty() {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn parse_marker_id(trimmed: &str, prefix: &str) -> Option<String> {
    let inner = trimmed.strip_prefix(prefix)?.strip_suffix("*/")?;
    if inner.is_empty() {
        return None;
    }
    Some(inner.to_string())
}

#[derive(Debug, Clone)]
struct GenBeginMarker {
    id: String,
    version: String,
    rust_sha256: String,
}

fn parse_gen_begin_marker(trimmed: &str) -> Option<GenBeginMarker> {
    let inner = trimmed.strip_prefix(GEN_BEGIN_PREFIX)?.strip_suffix("*/")?;
    let mut id: Option<String> = None;
    let mut version: Option<String> = None;
    let mut rust_sha256: Option<String> = None;

    for token in inner.split_whitespace() {
        let (k, v) = token.split_once('=')?;
        match k {
            "id" => id = Some(v.to_string()),
            "version" => version = Some(v.to_string()),
            "rust_sha256" => rust_sha256 = Some(v.to_string()),
            _ => return None,
        }
    }

    Some(GenBeginMarker {
        id: id?,
        version: version?,
        rust_sha256: rust_sha256?,
    })
}

fn is_if_directive(trimmed: &str) -> bool {
    trimmed.starts_with("#if")
}

fn is_endif_directive(trimmed: &str) -> bool {
    trimmed == ENDIF_DIRECTIVE
}

fn extract_rust_payload(region: &str) -> Result<String, String> {
    for line in region.lines() {
        if line.trim() == ELSE_DIRECTIVE {
            return Err("legacy `#else` inline layout is unsupported".to_string());
        }
    }
    if region.contains(RUST_BEGIN_PREFIX) || region.contains(RUST_END_PREFIX) {
        return Err("legacy `RUST-BEGIN/RUST-END` markers are unsupported".to_string());
    }
    if region.trim_start().starts_with(LEGACY_AT_RUST_PREFIX) {
        return Err("legacy `@rust { ... }` wrapper is unsupported".to_string());
    }
    Ok(region.to_string())
}

fn normalize_rust_payload(payload: &str) -> String {
    payload
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .to_string()
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{:02x}", byte);
    }
    out
}

#[derive(Debug, Clone)]
struct GenRegion {
    end_line: usize,
    id: String,
    version: String,
    rust_sha256: String,
}

fn parse_gen_region_from_first_nonempty(
    path: &Path,
    content: &str,
    lines: &[LineSpan],
    start_idx: usize,
) -> Result<Option<GenRegion>, String> {
    let begin_idx = match next_nonempty_line(lines, content, start_idx) {
        Some(idx) => idx,
        None => return Ok(None),
    };
    let begin_trimmed = line_trimmed(content, &lines[begin_idx]);
    let marker = match parse_gen_begin_marker(begin_trimmed) {
        Some(m) => m,
        None => return Ok(None),
    };

    let mut end_idx: Option<usize> = None;
    for (i, line) in lines.iter().enumerate().skip(begin_idx + 1) {
        let trimmed = line_trimmed(content, line);
        if let Some(end_id) = parse_marker_id(trimmed, GEN_END_PREFIX) {
            if end_id != marker.id {
                return Err(format!(
                    "{}:{}: GEN end marker id mismatch (begin={}, end={})",
                    path.display(),
                    i + 1,
                    marker.id,
                    end_id
                ));
            }
            end_idx = Some(i);
            break;
        }
    }
    let end_idx = end_idx.ok_or_else(|| {
        format!(
            "{}:{}: missing GEN end marker for id={}",
            path.display(),
            begin_idx + 1,
            marker.id
        )
    })?;

    Ok(Some(GenRegion {
        end_line: end_idx,
        id: marker.id,
        version: marker.version,
        rust_sha256: marker.rust_sha256,
    }))
}

fn make_auto_id(path: &Path, block_index: usize) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("inline_block");
    let mut sanitized = String::with_capacity(stem.len());
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    if sanitized.is_empty() {
        sanitized.push_str("inline_block");
    }
    format!("{}.{}", sanitized, block_index)
}

#[derive(Debug, Clone)]
struct ParsedBlock {
    if_line: usize,
    id: String,
    rust_hash: String,
    rust_payload_normalized: String,
    if_indent: String,
    replace_start: usize,
    replace_end: usize,
    generated_region: Option<GenRegion>,
}

fn parse_blocks(path: &Path, content: &str) -> Result<Vec<ParsedBlock>, String> {
    let lines = collect_line_spans(content);
    let mut blocks = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut i = 0usize;
    while i < lines.len() {
        if line_trimmed(content, &lines[i]) != IF_RUSTYCPP_RUST {
            i += 1;
            continue;
        }

        let mut depth = 0usize;
        let mut else_idx: Option<usize> = None;
        let mut endif_idx: Option<usize> = None;
        for j in i + 1..lines.len() {
            let trimmed = line_trimmed(content, &lines[j]);
            if is_if_directive(trimmed) {
                depth += 1;
                continue;
            }
            if is_endif_directive(trimmed) {
                if depth == 0 {
                    endif_idx = Some(j);
                    break;
                }
                depth -= 1;
                continue;
            }
            if trimmed == ELSE_DIRECTIVE && depth == 0 && else_idx.is_none() {
                else_idx = Some(j);
            }
        }
        let endif_idx = endif_idx.ok_or_else(|| {
            format!(
                "{}:{}: missing matching `{}` for `{}`",
                path.display(),
                i + 1,
                ENDIF_DIRECTIVE,
                IF_RUSTYCPP_RUST
            )
        })?;

        if let Some(else_line) = else_idx {
            return Err(format!(
                "{}:{}: legacy `#else` inline layout is unsupported; use `#if RUSTYCPP_RUST ... #endif` followed by `GEN` markers",
                path.display(),
                else_line + 1
            ));
        }

        let rust_region_start = lines[i].end;
        let rust_region_end = lines[endif_idx].start;
        let rust_region = &content[rust_region_start..rust_region_end];
        let extracted = extract_rust_payload(rust_region)
            .map_err(|e| format!("{}:{}: invalid Rust payload: {}", path.display(), i + 1, e))?;
        let rust_payload_normalized = normalize_rust_payload(&extracted);
        let rust_hash = sha256_hex(&rust_payload_normalized);

        let generated_region =
            parse_gen_region_from_first_nonempty(path, content, &lines, endif_idx + 1)?;
        let id_from_gen = generated_region.as_ref().map(|g| g.id.clone());
        let id = match id_from_gen {
            Some(id) => id,
            None => make_auto_id(path, blocks.len() + 1),
        };

        if !seen_ids.insert(id.clone()) {
            return Err(format!(
                "{}:{}: duplicate inline block id={}",
                path.display(),
                i + 1,
                id
            ));
        }

        let replace_start = lines[i].start;
        let replace_end = if let Some(existing_gen) = &generated_region {
            lines[existing_gen.end_line].end
        } else {
            lines[endif_idx].end
        };

        blocks.push(ParsedBlock {
            if_line: i + 1,
            id,
            rust_hash,
            rust_payload_normalized,
            if_indent: line_indent(content, &lines[i]),
            replace_start,
            replace_end,
            generated_region,
        });

        i = endif_idx + 1;
    }

    Ok(blocks)
}

/// Every `enum` declared by ANY inline-Rust block in this file.
///
/// Blocks are transpiled one at a time, so a block that merely *uses* an
/// enum declared in a sibling block has no way to know it is a local
/// C-like enum. Codegen's CamelCase fallback then guesses it is an
/// externally-transpiled data enum and emits `E::Variant()` — a call on
/// an enumerator, which does not compile. In Rust every item in a module
/// is visible to every other, so feeding the siblings back in via the
/// existing `cross_file_enums` seam restores the real language rule.
fn collect_file_enums(blocks: &[ParsedBlock]) -> Vec<syn::ItemEnum> {
    let mut enums = Vec::new();
    for block in blocks {
        let Ok(file) = syn::parse_file(&block.rust_payload_normalized) else {
            continue;
        };
        for item in file.items {
            if let syn::Item::Enum(item_enum) = item {
                enums.push(item_enum);
            }
        }
    }
    enums
}

fn render_generated_region(
    block: &ParsedBlock,
    file_enums: &[syn::ItemEnum],
) -> Result<String, String> {
    let generated_cpp = transpile_payload_to_cpp(block, file_enums)?;
    Ok(render_generated_region_with_cpp(block, &generated_cpp))
}

fn transpile_payload_to_cpp(
    block: &ParsedBlock,
    file_enums: &[syn::ItemEnum],
) -> Result<String, String> {
    let options = transpile::TranspileOptions {
        // Sibling blocks in the same file are this block's module scope.
        cross_file_enums: file_enums.to_vec(),
        // Inline-rust blocks are spliced into a TU that `import rusty;`, and may
        // sit inside a consumer namespace — suppress the redundant runtime
        // preamble that would otherwise shadow `::rusty`.
        inline_rust_block: true,
        ..transpile::TranspileOptions::default()
    };
    let generated = transpile::transpile_full_with_options(
        &block.rust_payload_normalized,
        None,
        &UserTypeMap::default(),
        &HashSet::new(),
        None,
        &options,
    )?;
    Ok(strip_inline_prelude(&generated))
}

fn strip_inline_prelude(generated_cpp: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    let mut skipping_gnu_pragma_block = false;
    for line in generated_cpp.lines() {
        let trimmed = line.trim();
        if skipping_gnu_pragma_block {
            if trimmed == "#endif" {
                skipping_gnu_pragma_block = false;
            }
            continue;
        }
        if trimmed == "// Auto-generated by rusty-cpp-transpiler" {
            continue;
        }
        if trimmed == "// Do not edit manually." {
            continue;
        }
        if trimmed.starts_with("#include ") {
            continue;
        }
        if trimmed == "#if defined(__GNUC__)" {
            skipping_gnu_pragma_block = true;
            continue;
        }
        lines.push(line);
    }

    let start = lines
        .iter()
        .position(|line| !line.trim().is_empty())
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .map(|idx| idx + 1)
        .unwrap_or(start);
    lines[start..end].join("\n")
}

fn render_generated_region_with_cpp(block: &ParsedBlock, generated_cpp: &str) -> String {
    let mut out = String::new();
    let prefix = &block.if_indent;
    out.push_str(prefix);
    out.push_str("/*RUSTYCPP:GEN-BEGIN id=");
    out.push_str(&block.id);
    out.push_str(" version=1 rust_sha256=");
    out.push_str(&block.rust_hash);
    out.push_str("*/\n");
    for line in generated_cpp.lines() {
        out.push_str(prefix);
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(prefix);
    out.push_str("/*RUSTYCPP:GEN-END id=");
    out.push_str(&block.id);
    out.push_str("*/\n");
    out
}

fn render_rust_block(block: &ParsedBlock) -> String {
    let mut out = String::new();
    let prefix = &block.if_indent;
    out.push_str(prefix);
    out.push_str(IF_RUSTYCPP_RUST);
    out.push('\n');
    if !block.rust_payload_normalized.is_empty() {
        out.push_str(&block.rust_payload_normalized);
        if !block.rust_payload_normalized.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str(prefix);
    out.push_str(ENDIF_DIRECTIVE);
    out.push('\n');
    out
}

fn render_block_rewrite(
    block: &ParsedBlock,
    file_enums: &[syn::ItemEnum],
) -> Result<String, String> {
    let mut out = String::new();
    out.push_str(&render_rust_block(block));
    out.push_str(&render_generated_region(block, file_enums)?);
    Ok(out)
}

const DISPATCH_BEGIN: &str = "/*RUSTYCPP:GEN-DISPATCH-BEGIN*/";
const DISPATCH_END: &str = "/*RUSTYCPP:GEN-DISPATCH-END*/";
const DISPATCH_TRAILER: &str = "// namespace rusty::detail (issue #31 deref_call dispatch)";

/// Cut the `namespace rusty { namespace detail { RUSTY_METHOD_DISPATCH(..) } }`
/// block out of a generated region, returning the dispatched method names.
///
/// Issue #33: an inline-rust block is spliced wherever the `#if RUSTYCPP_RUST`
/// sits — typically INSIDE a consumer namespace. Emitting the functor there
/// declares `demo::rusty`, which then shadows `::rusty` for the rest of the
/// block, so every later `rusty::deref_call` / `rusty::clone` / `rusty::thread`
/// resolves into the wrong namespace. The definitions have to live at global
/// scope, exactly as module output already places them.
fn take_dispatch_functor_block(region: &mut String) -> Vec<String> {
    let Some(start) = region.find("namespace rusty { namespace detail {") else {
        return Vec::new();
    };
    let Some(trailer) = region[start..].find(DISPATCH_TRAILER) else {
        return Vec::new();
    };
    let mut end = start + trailer + DISPATCH_TRAILER.len();
    // consume the rest of the trailer line plus any blank lines it left behind
    while end < region.len() && region.as_bytes()[end] != b'\n' {
        end += 1;
    }
    while end < region.len() && region.as_bytes()[end] == b'\n' {
        end += 1;
        if !region[end..].starts_with('\n') {
            break;
        }
    }
    let cut = region[start..end].to_string();
    let mut names = Vec::new();
    for line in cut.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("RUSTY_METHOD_DISPATCH(")
            && let Some(name) = rest.strip_suffix(')')
        {
            names.push(name.to_string());
        }
    }
    region.replace_range(start..end, "");
    names
}

/// Drop a previously hoisted dispatch region so `--rewrite` stays idempotent.
fn strip_hoisted_dispatch_region(content: &str) -> String {
    let Some(start) = content.find(DISPATCH_BEGIN) else {
        return content.to_string();
    };
    let Some(rel_end) = content[start..].find(DISPATCH_END) else {
        return content.to_string();
    };
    let mut end = start + rel_end + DISPATCH_END.len();
    while end < content.len() && content.as_bytes()[end] == b'\n' {
        end += 1;
    }
    let mut out = String::with_capacity(content.len());
    out.push_str(&content[..start]);
    out.push_str(&content[end..]);
    out
}

/// Insert the hoisted functor definitions at GLOBAL scope: after
/// `export module X;` and the imports that must immediately follow it,
/// otherwise after the leading `#include`/`module;` preamble.
fn insert_hoisted_dispatch_region(content: &str, methods: &[String]) -> String {
    if methods.is_empty() {
        return content.to_string();
    }
    let mut block = String::new();
    block.push_str(DISPATCH_BEGIN);
    block.push('\n');
    block.push_str("namespace rusty { namespace detail {\n");
    for m in methods {
        // Iterator DEFAULT methods have no member on a user impl that defines
        // only the required ones (`impl Iterator for X { fn next }` has no
        // `.fuse()`), so the member-only dispatcher cannot reach them. For the
        // names whose runtime counterpart is spelled IDENTICALLY
        // (`rusty::fuse`, ...), emit the variant that falls back to it.
        // Deliberately a small allowlist: the macro spells `rusty::<name>` as a
        // QUALIFIED id resolved at definition time, so a name without a
        // counterpart is ill-formed and breaks the whole module build. The
        // rest of the iterator family maps to DIFFERENTLY-named helpers
        // (`max` -> `rusty::iter_max`) and cannot use this form.
        let has_same_named_free_fn = matches!(
            m.as_str(),
            "fuse" | "peekable" | "inspect" | "skip_while" | "take_while"
        );
        block.push_str(if has_same_named_free_fn {
            "RUSTY_METHOD_DISPATCH_FREE("
        } else {
            "RUSTY_METHOD_DISPATCH("
        });
        block.push_str(m);
        block.push_str(")\n");
    }
    block.push_str("} } ");
    block.push_str(DISPATCH_TRAILER);
    block.push('\n');
    block.push_str(DISPATCH_END);
    block.push_str("\n\n");

    // Prefer just after `export module X;` + its import block: imports must
    // immediately follow the module declaration, so a namespace injected
    // before them would be ill-formed.
    let anchor = content
        .find("\nexport module ")
        .map(|i| i + 1)
        .or_else(|| content.starts_with("export module ").then_some(0));
    let mut pos = match anchor {
        Some(a) => content[a..]
            .find('\n')
            .map(|nl| a + nl + 1)
            .unwrap_or(content.len()),
        None => {
            // No module declaration — sit after the include/preamble prologue.
            let mut p = 0usize;
            for line in content.lines() {
                let t = line.trim();
                if t.starts_with("#include") || t.starts_with("module;") || t.is_empty() {
                    p += line.len() + 1;
                } else {
                    break;
                }
            }
            p.min(content.len())
        }
    };
    if anchor.is_some() {
        loop {
            let rest = &content[pos..];
            let line_end = rest
                .find('\n')
                .map(|nl| pos + nl + 1)
                .unwrap_or(content.len());
            let trimmed = content[pos..line_end].trim_start();
            if trimmed.starts_with("import ") || trimmed.starts_with("//") || trimmed.is_empty() {
                pos = line_end;
                if line_end >= content.len() {
                    break;
                }
            } else {
                break;
            }
        }
    }
    let mut out = String::with_capacity(content.len() + block.len());
    out.push_str(&content[..pos]);
    out.push_str(&block);
    out.push_str(&content[pos..]);
    out
}

fn rewrite_content(path: &Path, content: &str, blocks: &[ParsedBlock]) -> Result<String, String> {
    if blocks.is_empty() {
        return Ok(content.to_string());
    }

    let file_enums = collect_file_enums(blocks);

    let mut out = String::with_capacity(content.len() + blocks.len() * 128);
    let mut cursor = 0usize;
    let mut dispatch_methods: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for block in blocks {
        out.push_str(&content[cursor..block.replace_start]);
        let mut rewritten = render_block_rewrite(block, &file_enums).map_err(|e| {
            format!(
                "{}:{}: failed to transpile inline block id={}: {}",
                path.display(),
                block.if_line,
                block.id,
                e
            )
        })?;
        // Issue #33: hoist the dispatch functors out of the block — inside a
        // consumer namespace they would declare `demo::rusty` and shadow the
        // real `::rusty`.
        dispatch_methods.extend(take_dispatch_functor_block(&mut rewritten));
        out.push_str(&rewritten);
        cursor = block.replace_end;
    }
    out.push_str(&content[cursor..]);

    let out = strip_hoisted_dispatch_region(&out);
    let methods: Vec<String> = dispatch_methods.into_iter().collect();
    Ok(insert_hoisted_dispatch_region(&out, &methods))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post_endif_fixture(gen_hash: &str) -> String {
        format!(
            r#"#if RUSTYCPP_RUST
fn add(a: i32, b: i32) -> i32 {{
    a + b
}}
#endif
/*RUSTYCPP:GEN-BEGIN id=demo.add version=1 rust_sha256={}*/
// old generated text
/*RUSTYCPP:GEN-END id=demo.add*/
"#,
            gen_hash
        )
    }

    #[test]
    fn test_dispatch_functors_are_hoisted_out_of_the_consumer_namespace() {
        // Issue #33: the deref-dispatch functor was emitted inside the
        // generated block, which normally sits INSIDE a consumer namespace.
        // That declares `demo::rusty` and shadows `::rusty`, so every later
        // `rusty::deref_call` / `rusty::clone` / `rusty::thread` resolves into
        // the wrong namespace ("no member named 'deref_call' in namespace
        // 'demo::rusty'"). The definitions belong at global scope.
        let mut region = String::from(
            "namespace rusty { namespace detail {\nRUSTY_METHOD_DISPATCH(is_ready)\n} } // namespace rusty::detail (issue #31 deref_call dispatch)\n\ntemplate<typename W>\nbool f();\n",
        );
        let taken = take_dispatch_functor_block(&mut region);
        assert_eq!(taken, vec!["is_ready".to_string()]);
        assert!(
            !region.contains("namespace rusty"),
            "functor left inside the block: {region}"
        );
        assert!(region.contains("template<typename W>"), "body lost: {region}");

        let file = "module;\n#include <rusty/rusty.hpp>\nexport module m;\nimport std;\nexport namespace demo {\nbody\n}\n";
        let hoisted = insert_hoisted_dispatch_region(file, &["is_ready".to_string()]);
        let dispatch_at = hoisted.find("RUSTY_METHOD_DISPATCH").expect("no functor emitted");
        let ns_at = hoisted.find("export namespace demo").expect("namespace vanished");
        assert!(
            dispatch_at < ns_at,
            "functor must precede the consumer namespace:\n{hoisted}"
        );
        let import_at = hoisted.find("import std;").expect("import vanished");
        assert!(
            import_at < dispatch_at,
            "imports must still immediately follow the module declaration:\n{hoisted}"
        );

        // Re-running --rewrite must not stack regions.
        let again = insert_hoisted_dispatch_region(
            &strip_hoisted_dispatch_region(&hoisted),
            &["is_ready".to_string()],
        );
        assert_eq!(
            again.matches(DISPATCH_BEGIN).count(),
            1,
            "hoisted region duplicated on re-run:\n{again}"
        );
    }

    #[test]
    fn test_parse_blocks_extracts_hash_for_post_endif_layout() {
        let content = post_endif_fixture("deadbeef");
        let blocks = parse_blocks(Path::new("demo.hpp"), &content).expect("parse");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].id, "demo.add");
        assert_eq!(
            blocks[0]
                .generated_region
                .as_ref()
                .expect("gen")
                .rust_sha256,
            "deadbeef"
        );
        assert!(blocks[0].rust_payload_normalized.contains("fn add"));
        assert_eq!(blocks[0].rust_hash.len(), 64);
    }

    #[test]
    fn test_rewrite_content_updates_gen_hash_and_body() {
        let content = post_endif_fixture("deadbeef");
        let blocks = parse_blocks(Path::new("demo.hpp"), &content).expect("parse");
        let rewritten = rewrite_content(Path::new("demo.hpp"), &content, &blocks).expect("rewrite");
        assert!(rewritten.contains("int32_t add(int32_t a, int32_t b);"));
        assert!(rewritten.contains("int32_t add(int32_t a, int32_t b) {"));
        assert!(!rewritten.contains("#include <cstdint>"));
        assert!(!rewritten.contains("old generated text"));
        assert!(rewritten.contains(&format!("rust_sha256={}", blocks[0].rust_hash)));
    }

    /// A block-local fn calling another emitted `::helper(..)` — correct for
    /// single-file outputs whose fns really sit at purview scope, but an
    /// inline-rust GEN region lives inside the consumer's namespace, where
    /// `::helper` names nothing. Block-local crate-root calls must stay bare.
    #[test]
    fn test_block_local_fn_call_is_not_globally_qualified() {
        let content = r#"#if RUSTYCPP_RUST
fn helper(x: i32) -> i32 {
    x + 1
}
fn caller(x: i32) -> i32 {
    helper(x) * 2
}
#endif
"#;
        let blocks = parse_blocks(Path::new("demo.hpp"), content).expect("parse");
        let out = rewrite_content(Path::new("demo.hpp"), content, &blocks).expect("rewrite");
        assert!(
            !out.contains("::helper("),
            "block-local call must not be globally qualified:\n{out}"
        );
        assert!(
            out.contains("helper(std::move(x))"),
            "the call itself must survive:\n{out}"
        );
    }

    /// Issue #35: the guard deref was dropped whenever the receiver was
    /// concrete but untypable. `x.borrow_mut()` yields a `RefMut` guard in the
    /// C++ runtime, not the `&mut T` the type model claims, so calling a method
    /// straight on it does not compile — and binding it to `auto&` binds a
    /// non-const lvalue reference to a prvalue. #32 covered only the generic
    /// receiver.
    #[test]
    fn test_concrete_receiver_guard_keeps_its_deref() {
        let content = r#"#if RUSTYCPP_RUST
fn push_inline(h: &Holder) {
    h.q.borrow_mut().push_back(1);
}
fn push_bound(h: &Holder) {
    let mut g = h.q.borrow_mut();
    (*g).push_back(2);
}
#endif
"#;
        let blocks = parse_blocks(Path::new("demo.hpp"), content).expect("parse");
        let out = rewrite_content(Path::new("demo.hpp"), content, &blocks).expect("rewrite");

        assert!(
            out.contains("rusty::deref_call(h.q.borrow_mut()"),
            "the inline call must dispatch through the guard:\n{out}"
        );
        assert!(
            !out.contains("auto& g ="),
            "a guard is a prvalue; `auto&` cannot bind it:\n{out}"
        );
        assert!(
            out.contains("auto&& g ="),
            "bind the guard with `auto&&`, which also still aliases a real \
             reference:\n{out}"
        );
        assert!(
            out.contains("deref_if_pointer_like(g)"),
            "`*g` must stay tolerant -- dropping it calls the method on the \
             guard:\n{out}"
        );
    }

    #[test]
    fn test_parse_blocks_rejects_legacy_else_layout() {
        let content = r#"#if RUSTYCPP_RUST
fn add(a: i32, b: i32) -> i32 {
    a + b
}
#else
/*RUSTYCPP:GEN-BEGIN id=demo.add version=1 rust_sha256=deadbeef*/
// old generated text
/*RUSTYCPP:GEN-END id=demo.add*/
#endif
"#;
        let err = parse_blocks(Path::new("demo.hpp"), content).expect_err("legacy should fail");
        assert!(err.contains("legacy `#else` inline layout is unsupported"));
    }

    #[test]
    fn test_parse_blocks_rejects_legacy_rust_begin_marker_layout() {
        let content = r#"#if RUSTYCPP_RUST
/*RUSTYCPP:RUST-BEGIN id=demo.add*/
fn add(a: i32, b: i32) -> i32 {
    a + b
}
/*RUSTYCPP:RUST-END id=demo.add*/
#endif
/*RUSTYCPP:GEN-BEGIN id=demo.add version=1 rust_sha256=deadbeef*/
// old generated text
/*RUSTYCPP:GEN-END id=demo.add*/
"#;
        let err = parse_blocks(Path::new("demo.hpp"), content).expect_err("legacy should fail");
        assert!(err.contains("legacy `RUST-BEGIN/RUST-END` markers are unsupported"));
    }

    #[test]
    fn test_parse_blocks_rejects_duplicate_ids() {
        let single = post_endif_fixture("abc");
        let dup = format!("{}\n{}", single, single);
        let err = parse_blocks(Path::new("dup.hpp"), &dup).expect_err("duplicate should fail");
        assert!(err.contains("duplicate inline block id=demo.add"));
    }
}
