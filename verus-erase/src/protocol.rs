// The stdin/stdout protocol of the `rusty-cpp-verus-erase` helper binary.
//
// Why a separate process
// ----------------------
// `verus_syn` depends on proc-macro2 with the `span-locations` feature. Linked
// into the transpiler, Cargo's feature unification turns that feature on for
// the transpiler's own proc-macro2 too, which makes every token it lexes carry
// line/column data: on SRPC's crate, with `--verus-exec` off, peak RSS went
// from 84 to 173 MB and CPU time from 56 to 72 s. The transpiler therefore
// does not link this crate. It spawns this helper, only under `--verus-exec`
// and only for a file that contains Verus constructs, and exchanges token
// text with it.
//
// Wire format (all lengths are decimal byte counts, every line ends in `\n`)
// --------------------------------------------------------------------------
// request (stdin):
//     rusty-cpp-verus-erase/1
//     blocks <N>
//     then N times:  <len>\n<len bytes of block text>\n
// response (stdout), exit status 0:
//     rusty-cpp-verus-erase/1
//     verus_builtin_macros <VERUS_BUILTIN_MACROS_VERSION>
//     verus_git_rev <VERUS_GIT_REV>
//     blocks <N>
//     then N times:  ok <len>\n<len bytes of erased text>\n
//                or  err <len>\n<len bytes of message>\n
//
// A block text is `TokenStream::to_string()` of one item-level `verus!`
// invocation's input (the tokens inside its braces). An `ok` payload is
// `TokenStream::to_string()` of what `erase_items` returns for it; the caller
// re-lexes it. That round trip is exact for every token tree except a
// `Delimiter::None` group, which prints without delimiters; the helper
// therefore reports such output as an `err` instead of losing the grouping.
// A malformed request is a protocol error: nothing on stdout, the reason on
// stderr, exit status 2.
//
// The response is a pure function of the request: blocks are erased in order
// by one process, and nothing else (environment, time, paths) is read.

use std::str::FromStr;

use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// First line of every request and response.
pub const PROTOCOL: &str = "rusty-cpp-verus-erase/1";

/// Serve one request (the whole of stdin) and return the whole response.
pub fn serve(request: &[u8]) -> Result<Vec<u8>, String> {
    let blocks = decode_request(request)?;
    let mut response = Vec::new();
    push_line(&mut response, PROTOCOL);
    push_line(
        &mut response,
        &format!("verus_builtin_macros {}", crate::VERUS_BUILTIN_MACROS_VERSION),
    );
    push_line(&mut response, &format!("verus_git_rev {}", crate::VERUS_GIT_REV));
    push_line(&mut response, &format!("blocks {}", blocks.len()));
    for block in &blocks {
        match erase_block_text(block) {
            Ok(erased) => push_payload(&mut response, "ok", &erased),
            Err(message) => push_payload(&mut response, "err", &message),
        }
    }
    Ok(response)
}

/// The `--version` line of the helper binary.
pub fn version_line() -> String {
    format!(
        "rusty-cpp-verus-erase {} verus_builtin_macros {} verus_git_rev {}",
        env!("CARGO_PKG_VERSION"),
        crate::VERUS_BUILTIN_MACROS_VERSION,
        crate::VERUS_GIT_REV
    )
}

/// Erase one block given as token text.
fn erase_block_text(text: &str) -> Result<String, String> {
    let tokens = TokenStream::from_str(text)
        .map_err(|error| format!("the verus! block does not lex: {error}"))?;
    let erased = crate::erase_items(tokens)?;
    if contains_invisible_group(&erased) {
        return Err(
            "the erasure contains an invisible (None-delimited) group, which token text cannot carry"
                .to_string(),
        );
    }
    Ok(erased.to_string())
}

fn contains_invisible_group(tokens: &TokenStream) -> bool {
    tokens.clone().into_iter().any(|tree| match tree {
        TokenTree::Group(group) => {
            group.delimiter() == Delimiter::None || contains_invisible_group(&group.stream())
        }
        _ => false,
    })
}

fn push_line(out: &mut Vec<u8>, line: &str) {
    out.extend_from_slice(line.as_bytes());
    out.push(b'\n');
}

fn push_payload(out: &mut Vec<u8>, tag: &str, payload: &str) {
    push_line(out, &format!("{tag} {}", payload.len()));
    out.extend_from_slice(payload.as_bytes());
    out.push(b'\n');
}

/// A cursor over the request bytes.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn line(&mut self) -> Result<&'a str, String> {
        let rest = &self.bytes[self.at..];
        let end = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(|| format!("truncated request at byte {}", self.at))?;
        self.at += end + 1;
        std::str::from_utf8(&rest[..end]).map_err(|_| "request line is not UTF-8".to_string())
    }

    fn payload(&mut self, len: usize) -> Result<&'a str, String> {
        let rest = &self.bytes[self.at..];
        if rest.len() < len + 1 || rest[len] != b'\n' {
            return Err(format!("truncated or unterminated payload at byte {}", self.at));
        }
        self.at += len + 1;
        std::str::from_utf8(&rest[..len]).map_err(|_| "payload is not UTF-8".to_string())
    }
}

fn decode_request(request: &[u8]) -> Result<Vec<String>, String> {
    let mut reader = Reader { bytes: request, at: 0 };
    let magic = reader.line()?;
    if magic != PROTOCOL {
        return Err(format!("expected `{PROTOCOL}`, got `{magic}`"));
    }
    let count = reader
        .line()?
        .strip_prefix("blocks ")
        .and_then(|count| count.parse::<usize>().ok())
        .ok_or("expected `blocks <N>`")?;
    let mut blocks = Vec::with_capacity(count);
    for _ in 0..count {
        let len = reader
            .line()?
            .parse::<usize>()
            .map_err(|_| "expected a block length".to_string())?;
        blocks.push(reader.payload(len)?.to_string());
    }
    if reader.at != request.len() {
        return Err(format!("{} trailing byte(s) after the last block", request.len() - reader.at));
    }
    Ok(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(blocks: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        push_line(&mut out, PROTOCOL);
        push_line(&mut out, &format!("blocks {}", blocks.len()));
        for block in blocks {
            push_line(&mut out, &block.len().to_string());
            out.extend_from_slice(block.as_bytes());
            out.push(b'\n');
        }
        out
    }

    #[test]
    fn response_carries_revision_and_one_result_per_block_in_order() {
        let response = serve(&request(&[
            "fn a() ensures true {}",
            "fn broken() -> {}",
            "spec fn s() -> int { 0 } fn b(x: u8) -> u8 { x }",
        ]))
        .unwrap();
        let text = String::from_utf8(response).unwrap();
        let mut lines = text.split('\n');
        assert_eq!(lines.next(), Some(PROTOCOL));
        assert_eq!(
            lines.next().unwrap(),
            format!("verus_builtin_macros {}", crate::VERUS_BUILTIN_MACROS_VERSION)
        );
        assert_eq!(lines.next().unwrap(), format!("verus_git_rev {}", crate::VERUS_GIT_REV));
        assert_eq!(lines.next(), Some("blocks 3"));
        let first = TokenStream::from_str("fn a() {}").unwrap().to_string();
        assert_eq!(lines.next().unwrap(), format!("ok {}", first.len()));
        assert_eq!(lines.next().unwrap(), first);
        assert!(lines.next().unwrap().starts_with("err "));
        let rest = lines.collect::<Vec<_>>().join("\n");
        let third = TokenStream::from_str("fn b(x: u8) -> u8 { x }").unwrap().to_string();
        assert!(rest.ends_with(&format!("ok {}\n{}\n", third.len(), third)), "{rest}");
    }

    #[test]
    fn response_is_deterministic() {
        let input = request(&["pub fn f(x: u64) -> (r: u64) requires x < 10 ensures r == x { x }"]);
        assert_eq!(serve(&input).unwrap(), serve(&input).unwrap());
    }

    #[test]
    fn malformed_requests_are_protocol_errors() {
        assert!(serve(b"").is_err());
        assert!(serve(b"other/1\nblocks 0\n").is_err());
        assert!(serve(format!("{PROTOCOL}\nblocks 1\n5\nfn\n").as_bytes()).is_err());
        assert!(serve(format!("{PROTOCOL}\nblocks 0\nextra").as_bytes()).is_err());
        assert!(serve(format!("{PROTOCOL}\nblocks 0\n").as_bytes()).is_ok());
    }

    #[test]
    fn unlexable_block_is_an_err_payload() {
        let text = String::from_utf8(serve(&request(&["fn f() { \"unterminated }"])).unwrap()).unwrap();
        assert!(text.contains("\nerr "), "{text}");
    }
}
