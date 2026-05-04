use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const IF_RUSTYCPP_RUST: &str = "#if RUSTYCPP_RUST";
const ELSE_DIRECTIVE: &str = "#else";
const ENDIF_DIRECTIVE: &str = "#endif";
const RUST_BEGIN_PREFIX: &str = "/*RUSTYCPP:RUST-BEGIN id=";
const RUST_END_PREFIX: &str = "/*RUSTYCPP:RUST-END id=";
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
            let rewritten = rewrite_content(&content, &blocks);
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

#[derive(Debug, Clone)]
struct ExtractedRustPayload {
    payload: String,
    marker_id: Option<String>,
}

fn unwrap_optional_at_rust_wrapper(region: &str) -> Result<String, String> {
    let trimmed = region.trim();
    if !trimmed.starts_with("@rust") {
        return Ok(region.to_string());
    }
    let at = region
        .find("@rust")
        .ok_or_else(|| "invalid @rust wrapper".to_string())?;
    let tail = &region[at + "@rust".len()..];
    let open_rel = tail
        .find('{')
        .ok_or_else(|| "missing `{` after `@rust`".to_string())?;
    let after_open = at + "@rust".len() + open_rel + 1;
    let close_abs = region
        .rfind('}')
        .ok_or_else(|| "missing closing `}` for `@rust { ... }`".to_string())?;
    if close_abs < after_open {
        return Err("invalid `@rust` block braces".to_string());
    }
    let trailing = &region[close_abs + 1..];
    if !trailing.trim().is_empty() {
        return Err("unexpected trailing content after `@rust { ... }`".to_string());
    }
    Ok(region[after_open..close_abs].to_string())
}

fn extract_rust_payload(region: &str) -> Result<ExtractedRustPayload, String> {
    let lines = collect_line_spans(region);
    let first_idx = match next_nonempty_line(&lines, region, 0) {
        Some(idx) => idx,
        None => {
            return Ok(ExtractedRustPayload {
                payload: String::new(),
                marker_id: None,
            });
        }
    };
    let first = line_trimmed(region, &lines[first_idx]);
    if let Some(begin_id) = parse_marker_id(first, RUST_BEGIN_PREFIX) {
        let mut end_idx: Option<usize> = None;
        for (i, line) in lines.iter().enumerate().skip(first_idx + 1) {
            let trimmed = line_trimmed(region, line);
            if let Some(end_id) = parse_marker_id(trimmed, RUST_END_PREFIX) {
                if end_id != begin_id {
                    return Err(format!(
                        "Rust end marker id mismatch (begin={}, end={})",
                        begin_id, end_id
                    ));
                }
                end_idx = Some(i);
                break;
            }
        }
        let end_idx = end_idx.ok_or_else(|| "missing Rust end marker".to_string())?;
        let inner = &region[lines[first_idx].end..lines[end_idx].start];
        let payload = unwrap_optional_at_rust_wrapper(inner)?;
        return Ok(ExtractedRustPayload {
            payload,
            marker_id: Some(begin_id),
        });
    }

    let payload = unwrap_optional_at_rust_wrapper(region)?;
    Ok(ExtractedRustPayload {
        payload,
        marker_id: None,
    })
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

        let rust_region_start = lines[i].end;
        let rust_region_end = else_idx
            .map(|idx| lines[idx].start)
            .unwrap_or(lines[endif_idx].start);
        let rust_region = &content[rust_region_start..rust_region_end];
        let extracted = extract_rust_payload(rust_region).map_err(|e| {
            format!(
                "{}:{}: invalid Rust payload: {}",
                path.display(),
                i + 1,
                e
            )
        })?;
        let rust_payload_normalized = normalize_rust_payload(&extracted.payload);
        let rust_hash = sha256_hex(&rust_payload_normalized);

        let generated_region = if let Some(else_line) = else_idx {
            parse_gen_region_from_first_nonempty(path, content, &lines, else_line + 1)?
        } else {
            parse_gen_region_from_first_nonempty(path, content, &lines, endif_idx + 1)?
        };

        let id_from_marker = extracted.marker_id;
        let id_from_gen = generated_region.as_ref().map(|g| g.id.clone());
        let id = match (id_from_marker, id_from_gen) {
            (Some(a), Some(b)) => {
                if a != b {
                    return Err(format!(
                        "{}:{}: id mismatch between Rust marker ({}) and GEN marker ({})",
                        path.display(),
                        i + 1,
                        a,
                        b
                    ));
                }
                a
            }
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => make_auto_id(path, blocks.len() + 1),
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

fn render_generated_region(block: &ParsedBlock) -> String {
    let mut out = String::new();
    let prefix = &block.if_indent;
    out.push_str(prefix);
    out.push_str("/*RUSTYCPP:GEN-BEGIN id=");
    out.push_str(&block.id);
    out.push_str(" version=1 rust_sha256=");
    out.push_str(&block.rust_hash);
    out.push_str("*/\n");
    out.push_str(prefix);
    out.push_str("// generated by rusty-cpp-transpiler inline-rust v1\n");
    out.push_str(prefix);
    out.push_str("// inline-rust-v1: fallback stub; semantic lowering is not yet implemented\n");
    out.push_str(prefix);
    out.push_str("// rust payload (normalized) line count: ");
    out.push_str(&block.rust_payload_normalized.lines().count().to_string());
    out.push('\n');
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

fn render_block_rewrite(block: &ParsedBlock) -> String {
    let mut out = String::new();
    out.push_str(&render_rust_block(block));
    out.push_str(&render_generated_region(block));
    out
}

fn rewrite_content(content: &str, blocks: &[ParsedBlock]) -> String {
    if blocks.is_empty() {
        return content.to_string();
    }

    let mut out = String::with_capacity(content.len() + blocks.len() * 128);
    let mut cursor = 0usize;
    for block in blocks {
        out.push_str(&content[cursor..block.replace_start]);
        out.push_str(&render_block_rewrite(block));
        cursor = block.replace_end;
    }
    out.push_str(&content[cursor..]);
    out
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

    fn legacy_else_fixture(gen_hash: &str) -> String {
        format!(
            r#"#if RUSTYCPP_RUST
/*RUSTYCPP:RUST-BEGIN id=demo.add*/
@rust {{
fn add(a: i32, b: i32) -> i32 {{
    a + b
}}
}}
/*RUSTYCPP:RUST-END id=demo.add*/
#else
/*RUSTYCPP:GEN-BEGIN id=demo.add version=1 rust_sha256={}*/
// old generated text
/*RUSTYCPP:GEN-END id=demo.add*/
#endif
"#,
            gen_hash
        )
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
        let rewritten = rewrite_content(&content, &blocks);
        assert!(rewritten.contains("generated by rusty-cpp-transpiler inline-rust v1"));
        assert!(rewritten.contains("inline-rust-v1: fallback stub"));
        assert!(!rewritten.contains("old generated text"));
        assert!(rewritten.contains(&format!("rust_sha256={}", blocks[0].rust_hash)));
    }

    #[test]
    fn test_rewrite_migrates_legacy_else_layout_to_post_endif() {
        let content = legacy_else_fixture("deadbeef");
        let blocks = parse_blocks(Path::new("demo.hpp"), &content).expect("parse");
        let rewritten = rewrite_content(&content, &blocks);
        assert!(!rewritten.contains("\n#else\n"));
        assert!(!rewritten.contains("RUSTYCPP:RUST-BEGIN"));
        assert!(!rewritten.contains("@rust {"));
        assert!(rewritten.contains("#endif\n/*RUSTYCPP:GEN-BEGIN"));
    }

    #[test]
    fn test_parse_blocks_rejects_duplicate_ids() {
        let single = post_endif_fixture("abc");
        let dup = format!("{}\n{}", single, single);
        let err = parse_blocks(Path::new("dup.hpp"), &dup).expect_err("duplicate should fail");
        assert!(err.contains("duplicate inline block id=demo.add"));
    }
}
