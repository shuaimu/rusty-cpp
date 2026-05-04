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
                if block.gen_version != "1" {
                    return Err(format!(
                        "{}:{}: unsupported GEN marker version {}; expected 1",
                        path.display(),
                        block.if_line,
                        block.gen_version
                    ));
                }
                if block.gen_hash != block.rust_hash {
                    return Err(format!(
                        "{}:{}: hash mismatch for id={} (marker={}, expected={})",
                        path.display(),
                        block.if_line,
                        block.id,
                        block.gen_hash,
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

fn extract_rust_payload(region: &str) -> Result<String, String> {
    let rust_at = region
        .find("@rust")
        .ok_or_else(|| "missing `@rust` in Rust region".to_string())?;
    let tail = &region[rust_at + "@rust".len()..];
    let open_rel = tail
        .find('{')
        .ok_or_else(|| "missing `{` after `@rust`".to_string())?;
    let after_open = rust_at + "@rust".len() + open_rel + 1;

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
struct ParsedBlock {
    id: String,
    if_line: usize,
    gen_version: String,
    gen_hash: String,
    rust_hash: String,
    rust_payload_normalized: String,
    gen_begin_line_start: usize,
    gen_end_line_end: usize,
    gen_indent: String,
}

fn parse_blocks(path: &Path, content: &str) -> Result<Vec<ParsedBlock>, String> {
    let lines = collect_line_spans(content);
    let mut out = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut i = 0usize;
    while i < lines.len() {
        if line_trimmed(content, &lines[i]) != IF_RUSTYCPP_RUST {
            i += 1;
            continue;
        }

        let if_line = i + 1;
        let rust_begin_idx = next_nonempty_line(&lines, content, i + 1).ok_or_else(|| {
            format!(
                "{}:{}: expected Rust begin marker after `{}`",
                path.display(),
                if_line,
                IF_RUSTYCPP_RUST
            )
        })?;
        let rust_begin_line = line_trimmed(content, &lines[rust_begin_idx]);
        let rust_begin_id =
            parse_marker_id(rust_begin_line, RUST_BEGIN_PREFIX).ok_or_else(|| {
                format!(
                    "{}:{}: expected `{}` marker",
                    path.display(),
                    rust_begin_idx + 1,
                    RUST_BEGIN_PREFIX
                )
            })?;

        let mut rust_end_idx: Option<usize> = None;
        for j in rust_begin_idx + 1..lines.len() {
            let trimmed = line_trimmed(content, &lines[j]);
            if let Some(end_id) = parse_marker_id(trimmed, RUST_END_PREFIX) {
                if end_id != rust_begin_id {
                    return Err(format!(
                        "{}:{}: Rust end marker id mismatch (begin={}, end={})",
                        path.display(),
                        j + 1,
                        rust_begin_id,
                        end_id
                    ));
                }
                rust_end_idx = Some(j);
                break;
            }
        }
        let rust_end_idx = rust_end_idx.ok_or_else(|| {
            format!(
                "{}:{}: missing Rust end marker for id={}",
                path.display(),
                rust_begin_idx + 1,
                rust_begin_id
            )
        })?;

        let rust_region = &content[lines[rust_begin_idx].end..lines[rust_end_idx].start];
        let rust_payload = extract_rust_payload(rust_region).map_err(|e| {
            format!(
                "{}:{}: invalid `@rust` region for id={}: {}",
                path.display(),
                rust_begin_idx + 1,
                rust_begin_id,
                e
            )
        })?;
        let rust_payload_normalized = normalize_rust_payload(&rust_payload);
        let rust_hash = sha256_hex(&rust_payload_normalized);

        let else_idx = next_nonempty_line(&lines, content, rust_end_idx + 1).ok_or_else(|| {
            format!(
                "{}:{}: expected `{}` after Rust end marker id={}",
                path.display(),
                rust_end_idx + 1,
                ELSE_DIRECTIVE,
                rust_begin_id
            )
        })?;
        if line_trimmed(content, &lines[else_idx]) != ELSE_DIRECTIVE {
            return Err(format!(
                "{}:{}: expected `{}` after Rust end marker id={}",
                path.display(),
                else_idx + 1,
                ELSE_DIRECTIVE,
                rust_begin_id
            ));
        }

        let gen_begin_idx = next_nonempty_line(&lines, content, else_idx + 1).ok_or_else(|| {
            format!(
                "{}:{}: expected GEN begin marker after `{}`",
                path.display(),
                else_idx + 1,
                ELSE_DIRECTIVE
            )
        })?;
        let gen_begin_line = line_trimmed(content, &lines[gen_begin_idx]);
        let gen_marker = parse_gen_begin_marker(gen_begin_line).ok_or_else(|| {
            format!(
                "{}:{}: expected valid GEN begin marker",
                path.display(),
                gen_begin_idx + 1
            )
        })?;
        if gen_marker.id != rust_begin_id {
            return Err(format!(
                "{}:{}: GEN begin id mismatch (rust={}, gen={})",
                path.display(),
                gen_begin_idx + 1,
                rust_begin_id,
                gen_marker.id
            ));
        }

        let mut gen_end_idx: Option<usize> = None;
        for j in gen_begin_idx + 1..lines.len() {
            let trimmed = line_trimmed(content, &lines[j]);
            if let Some(end_id) = parse_marker_id(trimmed, GEN_END_PREFIX) {
                if end_id != rust_begin_id {
                    return Err(format!(
                        "{}:{}: GEN end marker id mismatch (begin={}, end={})",
                        path.display(),
                        j + 1,
                        rust_begin_id,
                        end_id
                    ));
                }
                gen_end_idx = Some(j);
                break;
            }
        }
        let gen_end_idx = gen_end_idx.ok_or_else(|| {
            format!(
                "{}:{}: missing GEN end marker for id={}",
                path.display(),
                gen_begin_idx + 1,
                rust_begin_id
            )
        })?;

        let endif_idx = next_nonempty_line(&lines, content, gen_end_idx + 1).ok_or_else(|| {
            format!(
                "{}:{}: expected `{}` after GEN end marker id={}",
                path.display(),
                gen_end_idx + 1,
                ENDIF_DIRECTIVE,
                rust_begin_id
            )
        })?;
        if line_trimmed(content, &lines[endif_idx]) != ENDIF_DIRECTIVE {
            return Err(format!(
                "{}:{}: expected `{}` after GEN end marker id={}",
                path.display(),
                endif_idx + 1,
                ENDIF_DIRECTIVE,
                rust_begin_id
            ));
        }

        if !seen_ids.insert(rust_begin_id.clone()) {
            return Err(format!(
                "{}:{}: duplicate inline block id={}",
                path.display(),
                if_line,
                rust_begin_id
            ));
        }

        out.push(ParsedBlock {
            id: rust_begin_id,
            if_line,
            gen_version: gen_marker.version,
            gen_hash: gen_marker.rust_sha256,
            rust_hash,
            rust_payload_normalized,
            gen_begin_line_start: lines[gen_begin_idx].start,
            gen_end_line_end: lines[gen_end_idx].end,
            gen_indent: line_indent(content, &lines[gen_begin_idx]),
        });

        i = endif_idx + 1;
    }

    Ok(out)
}

fn render_generated_region(block: &ParsedBlock) -> String {
    let mut out = String::new();
    let prefix = &block.gen_indent;
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
    out.push_str("*/");
    out.push('\n');
    out
}

fn rewrite_content(content: &str, blocks: &[ParsedBlock]) -> String {
    if blocks.is_empty() {
        return content.to_string();
    }

    let mut out = String::with_capacity(content.len() + blocks.len() * 128);
    let mut cursor = 0usize;
    for block in blocks {
        out.push_str(&content[cursor..block.gen_begin_line_start]);
        out.push_str(&render_generated_region(block));
        cursor = block.gen_end_line_end;
    }
    out.push_str(&content[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(gen_hash: &str) -> String {
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
    fn test_parse_blocks_extracts_hash_and_payload() {
        let content = fixture("deadbeef");
        let blocks = parse_blocks(Path::new("demo.hpp"), &content).expect("parse");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].id, "demo.add");
        assert_eq!(blocks[0].gen_hash, "deadbeef");
        assert!(blocks[0].rust_payload_normalized.contains("fn add"));
        assert_eq!(blocks[0].rust_hash.len(), 64);
    }

    #[test]
    fn test_rewrite_content_updates_gen_hash_and_body() {
        let content = fixture("deadbeef");
        let blocks = parse_blocks(Path::new("demo.hpp"), &content).expect("parse");
        let rewritten = rewrite_content(&content, &blocks);
        assert!(rewritten.contains("generated by rusty-cpp-transpiler inline-rust v1"));
        assert!(rewritten.contains("inline-rust-v1: fallback stub"));
        assert!(!rewritten.contains("old generated text"));
        assert!(rewritten.contains(&format!("rust_sha256={}", blocks[0].rust_hash)));
    }

    #[test]
    fn test_parse_blocks_rejects_duplicate_ids() {
        let hash = "abc";
        let single = fixture(hash);
        let dup = format!("{}\n{}", single, single);
        let err = parse_blocks(Path::new("dup.hpp"), &dup).expect_err("duplicate should fail");
        assert!(err.contains("duplicate inline block id=demo.add"));
    }
}
