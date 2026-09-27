//! `--verus-exec`: transpile the executable code inside Verus `verus! { }`
//! blocks exactly as plain rustc compiles it.
//!
//! Plain rustc never sees Verus syntax. The `verus!` proc macro runs Verus's
//! own `EraseGhost::EraseAll` rewrite and hands rustc the executable items
//! only: spec and proof functions disappear, `requires`/`ensures`, `proof { }`
//! blocks and ghost `let`s vanish, and what remains is ordinary Rust. This
//! module reproduces that expansion at the source level, before any other
//! pass reads the file, by running the *same* erasure code
//! (`verus_erase`, vendored from `verus_builtin_macros`) on every item-level
//! `verus!` invocation and splicing the resulting items in its place.
//!
//! The erasure runs out of process, in the `rusty-cpp-verus-erase` helper
//! (`verus-erase/src/protocol.rs` has the wire format and the reason: linking
//! `verus_syn` would turn on proc-macro2 `span-locations` for the whole
//! transpiler). A file is sent to the helper only if it contains a `verus!`
//! block; one spawn carries every block of the file. The helper is found via
//! `--verus-erase-helper`, else `$RUSTY_CPP_VERUS_ERASE`, else next to this
//! executable, and it must report exactly the Verus revision this transpiler
//! was built against (`VERUS_GIT_REV`), or the run fails.
//!
//! It then evaluates the two cfgs the Verus driver sets and plain rustc never
//! does, `verus_keep_ghost` and `verus_keep_ghost_body`, as false everywhere
//! in the file (Lion uses `#![cfg_attr(verus_keep_ghost, verus::trusted)]`
//! outside `verus!`, and `#[cfg(verus_keep_ghost)]` items inside it).
//!
//! What it does not do is lower the ghost residue that survives erasure
//! (`use vstd::prelude::*`, `View` impls, `Ghost<T>` values, vstd executable
//! methods). That is the next stage's job; this stage only guarantees that
//! downstream passes see the Rust rustc sees.
//!
//! Fail-closed rules:
//! - a file that cannot be parsed, or a `verus!` block Verus's own rewrite
//!   rejects (`compile_error!` in its output), is an error;
//! - any other Verus macro anywhere in the file (`verus_impl!`,
//!   `verus_keep_ghost!`, `proof!`, `struct_with_invariants!`, a `verus!` in
//!   statement position, ...) is an error rather than a TODO slot;
//! - a `verus_keep_ghost` cfg in a position this pass does not evaluate
//!   (inside a macro invocation's tokens, on an expression, ...) is an error.
//!
//! A file without any of these constructs is returned byte for byte.

use std::borrow::Cow;
use std::collections::{BTreeMap, VecDeque};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;

use proc_macro2::{TokenStream, TokenTree};
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::visit_mut::VisitMut;
use syn::{Attribute, Item, Meta, Token};

/// The `verus_builtin_macros` version whose erasure `--verus-exec` runs. It
/// must equal `verus_erase::VERUS_BUILTIN_MACROS_VERSION`; the helper repeats
/// it in every response and a mismatch fails the run.
pub const VERUS_BUILTIN_MACROS_VERSION: &str = "0.0.0-2025-11-10-1957";
/// The verus-lang/verus commit that erasure was vendored from, spelled the way
/// Cargo records a git source's resolved commit in `Cargo.lock` (so a consumer
/// can compare it with its own `vstd` source). Must equal
/// `verus_erase::VERUS_GIT_REV`; checked against every helper response.
pub const VERUS_GIT_REV: &str = "db81a7496bfffeef3da8b30c306600ea51d2b0fa";

/// First line of every helper request and response
/// (`verus_erase::protocol::PROTOCOL`).
const HELPER_PROTOCOL: &str = "rusty-cpp-verus-erase/1";
/// File name of the helper executable.
const HELPER_NAME: &str = "rusty-cpp-verus-erase";
/// Environment variable naming the helper executable.
pub const HELPER_ENV: &str = "RUSTY_CPP_VERUS_ERASE";

/// `--verus-build-info`: the Verus revision this transpiler expects, as one
/// line of JSON. A separate flag rather than extra `--build-info` keys because
/// consumers (SRPC's gate) require `--build-info` to carry exactly
/// `git_hash` and `git_dirty`.
pub fn build_info_json() -> String {
    format!(
        r#"{{"verus_builtin_macros_version":"{VERUS_BUILTIN_MACROS_VERSION}","verus_git_rev":"{VERUS_GIT_REV}"}}"#
    )
}

/// cfg names the Verus driver sets and plain rustc never does.
const VERUS_DRIVER_CFGS: &[&str] = &["verus_keep_ghost", "verus_keep_ghost_body"];

/// `--verus-exec` configuration carried on `TranspileOptions`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VerusExecConfig {
    /// Apply the pre-pass to every crate source before any other pass reads it.
    pub enabled: bool,
    /// `--dump-verus-erasure`: write each source unit, as handed downstream,
    /// to `<dir>/<crate name>/<source identity>`.
    pub dump_dir: Option<PathBuf>,
    /// `--verus-erase-helper`: the `rusty-cpp-verus-erase` executable. When
    /// absent, `$RUSTY_CPP_VERUS_ERASE`, then the directory of the running
    /// executable (and its parent, for Cargo's `deps/` test binaries).
    pub helper: Option<PathBuf>,
}

/// Apply `--verus-exec` to one source text (identity when disabled).
pub fn prepare_source(config: &VerusExecConfig, source: String) -> Result<String, String> {
    if !config.enabled {
        return Ok(source);
    }
    match erase_source(config, &source)? {
        Cow::Borrowed(_) => Ok(source),
        Cow::Owned(erased) => Ok(erased),
    }
}

/// Write one prepared source unit to the `--dump-verus-erasure` directory.
pub fn dump_source(
    config: &VerusExecConfig,
    crate_name: &str,
    identity: &Path,
    prepared: &str,
) -> Result<(), String> {
    let Some(dir) = config.dump_dir.as_ref() else {
        return Ok(());
    };
    let path = dir.join(crate_name).join(identity);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    std::fs::write(&path, prepared)
        .map_err(|error| format!("could not write {}: {error}", path.display()))
}

/// Erase every item-level `verus!` block in `source` and evaluate the Verus
/// driver cfgs as false. Returns the input unchanged (borrowed) when there is
/// nothing to do.
pub fn erase_source<'a>(config: &VerusExecConfig, source: &'a str) -> Result<Cow<'a, str>, String> {
    // Cheap screen: every construct this pass acts on or rejects spells one
    // of these names. A file that merely mentions one (a comment, a
    // same-named user item) is parsed and audited, then returned unchanged.
    if !VERUS_DRIVER_CFGS
        .iter()
        .chain(VERUS_MACROS)
        .chain(VERUS_MACRO_CRATES)
        .any(|name| source.contains(name))
    {
        return Ok(Cow::Borrowed(source));
    }
    let mut file = syn::parse_file(source)
        .map_err(|error| format!("--verus-exec could not parse the source: {error}"))?;
    let mut changed = false;

    let mut eraser = Eraser::prefetch(config, &file)?;
    changed |= erase_items(&mut file.items, "crate", &mut eraser)?;

    let mut cfg = DriverCfgPass::default();
    cfg.visit_file_mut(&mut file);
    if let Some(error) = cfg.error {
        return Err(error);
    }
    changed |= cfg.changed;

    let mut audit = ResidueAudit::default();
    audit.visit_file(&file);
    if !audit.errors.is_empty() {
        return Err(audit.errors.join("; "));
    }

    if changed {
        Ok(Cow::Owned(prettyplease::unparse(&file)))
    } else {
        Ok(Cow::Borrowed(source))
    }
}

// ---------------------------------------------------------------------------
// verus! erasure

/// The item-level macro paths that name `verus_builtin_macros::verus`.
fn is_verus_items_macro(path: &syn::Path) -> bool {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    let segments = segments.iter().map(String::as_str).collect::<Vec<_>>();
    matches!(
        segments.as_slice(),
        ["verus"]
            | ["vstd", "prelude", "verus"]
            | ["vstd", "verus"]
            | ["verus_builtin_macros", "verus"]
            | ["builtin_macros", "verus"]
    ) && path
        .segments
        .iter()
        .all(|segment| segment.arguments.is_none())
}

/// Replace every `verus! { ... }` in `items` (and in inline `mod` bodies) with
/// the items Verus's EraseAll rewrite produces. Returns whether anything
/// changed.
fn erase_items(items: &mut Vec<Item>, scope: &str, eraser: &mut Eraser<'_>) -> Result<bool, String> {
    let mut changed = false;
    let mut index = 0;
    while index < items.len() {
        match &mut items[index] {
            Item::Macro(item_macro)
                if item_macro.ident.is_none() && is_verus_items_macro(&item_macro.mac.path) =>
            {
                if !item_macro.attrs.is_empty() {
                    return Err(format!(
                        "--verus-exec: attributes on a `verus!` invocation in `{scope}` are unsupported"
                    ));
                }
                let erased = eraser.erase(&item_macro.mac.tokens)?.map_err(|error| {
                    format!("--verus-exec: Verus rejected a `verus!` block in `{scope}`: {error}")
                })?;
                let erased = TokenStream::from_str(&erased).map_err(|error| {
                    format!(
                        "--verus-exec: the erasure of a `verus!` block in `{scope}` does not lex: {error}"
                    )
                })?;
                let erased: syn::File = syn::parse2(erased).map_err(|error| {
                    format!(
                        "--verus-exec: the erasure of a `verus!` block in `{scope}` is not Rust syn can parse: {error}"
                    )
                })?;
                if !erased.attrs.is_empty() {
                    return Err(format!(
                        "--verus-exec: the erasure of a `verus!` block in `{scope}` produced inner attributes"
                    ));
                }
                let replacement = erased.items;
                let count = replacement.len();
                items.splice(index..=index, replacement);
                changed = true;
                // Verus's rewrite already recursed into inline modules inside
                // the block; revisit them only so a nested `verus!` there is
                // handled like any other. A `verus!` at the spliced level
                // itself is not re-expanded (rustc would); `ResidueAudit`
                // rejects it.
                let end = index + count;
                while index < end {
                    if let Item::Mod(module) = &mut items[index] {
                        let name = format!("{scope}::{}", module.ident);
                        if let Some((_, content)) = &mut module.content {
                            changed |= erase_items(content, &name, eraser)?;
                        }
                    }
                    index += 1;
                }
                continue;
            }
            Item::Mod(module) => {
                let name = format!("{scope}::{}", module.ident);
                if let Some((_, content)) = &mut module.content {
                    changed |= erase_items(content, &name, eraser)?;
                }
            }
            _ => {}
        }
        index += 1;
    }
    Ok(changed)
}

// ---------------------------------------------------------------------------
// the rusty-cpp-verus-erase helper

/// Erases `verus!` block bodies through the helper process. Every block that
/// is visible in the file before erasure goes to the helper in one request
/// (`prefetch`); a block that only appears inside another block's erasure (a
/// `verus!` nested in an inline module) costs one more request.
struct Eraser<'a> {
    config: &'a VerusExecConfig,
    /// Results keyed by the block's token text, in request order.
    ready: BTreeMap<String, VecDeque<Result<String, String>>>,
}

impl<'a> Eraser<'a> {
    fn prefetch(config: &'a VerusExecConfig, file: &syn::File) -> Result<Self, String> {
        let mut blocks = Vec::new();
        collect_verus_blocks(&file.items, &mut blocks);
        let mut eraser = Eraser {
            config,
            ready: BTreeMap::new(),
        };
        if !blocks.is_empty() {
            let results = run_helper(config, &blocks)?;
            for (block, result) in blocks.into_iter().zip(results) {
                eraser.ready.entry(block).or_default().push_back(result);
            }
        }
        Ok(eraser)
    }

    /// The erasure of one block body: `Ok(Ok(text))` for the erased items as
    /// token text, `Ok(Err(message))` when Verus rejects the block, `Err` when
    /// the helper itself could not be run.
    fn erase(&mut self, tokens: &TokenStream) -> Result<Result<String, String>, String> {
        let text = tokens.to_string();
        if let Some(result) = self.ready.get_mut(&text).and_then(VecDeque::pop_front) {
            return Ok(result);
        }
        Ok(run_helper(self.config, std::slice::from_ref(&text))?.remove(0))
    }
}

fn collect_verus_blocks(items: &[Item], out: &mut Vec<String>) {
    for item in items {
        match item {
            Item::Macro(item_macro)
                if item_macro.ident.is_none() && is_verus_items_macro(&item_macro.mac.path) =>
            {
                out.push(item_macro.mac.tokens.to_string());
            }
            Item::Mod(module) => {
                if let Some((_, content)) = &module.content {
                    collect_verus_blocks(content, out);
                }
            }
            _ => {}
        }
    }
}

/// Locate the helper executable (see `VerusExecConfig::helper`).
fn helper_path(config: &VerusExecConfig) -> Result<PathBuf, String> {
    if let Some(path) = &config.helper {
        return Ok(path.clone());
    }
    if let Some(path) = std::env::var_os(HELPER_ENV).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    let file_name = format!("{HELPER_NAME}{}", std::env::consts::EXE_SUFFIX);
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let mut candidates = vec![dir.join(&file_name)];
        // Cargo test binaries run from `target/<profile>/deps/`.
        if dir.file_name().is_some_and(|name| name == "deps")
            && let Some(profile_dir) = dir.parent()
        {
            candidates.push(profile_dir.join(&file_name));
        }
        if let Some(found) = candidates.into_iter().find(|candidate| candidate.is_file()) {
            return Ok(found);
        }
    }
    Err(format!(
        "--verus-exec runs Verus's erasure in the `{HELPER_NAME}` helper, and none was found \
         next to this executable. Build it in its own Cargo invocation \
         (`cargo build --release -p verus-erase`; building it together with the \
         transpiler turns proc-macro2 `span-locations` on for the transpiler), then \
         place it next to rusty-cpp-transpiler, set {HELPER_ENV}, or pass \
         --verus-erase-helper PATH"
    ))
}

fn push_line(out: &mut Vec<u8>, line: &str) {
    out.extend_from_slice(line.as_bytes());
    out.push(b'\n');
}

fn encode_request(blocks: &[String]) -> Vec<u8> {
    let mut request = Vec::new();
    push_line(&mut request, HELPER_PROTOCOL);
    push_line(&mut request, &format!("blocks {}", blocks.len()));
    for block in blocks {
        push_line(&mut request, &block.len().to_string());
        request.extend_from_slice(block.as_bytes());
        request.push(b'\n');
    }
    request
}

/// Decode a helper response carrying `expected` results, checking that the
/// helper vendors exactly the Verus revision this transpiler expects.
fn decode_response(
    response: &[u8],
    expected: usize,
) -> Result<Vec<Result<String, String>>, String> {
    let mut at = 0usize;
    let line = |at: &mut usize| -> Result<String, String> {
        let rest = &response[*at..];
        let end = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or("truncated response")?;
        *at += end + 1;
        String::from_utf8(rest[..end].to_vec()).map_err(|_| "response line is not UTF-8".to_string())
    };
    let magic = line(&mut at)?;
    if magic != HELPER_PROTOCOL {
        return Err(format!("expected `{HELPER_PROTOCOL}`, got `{magic}`"));
    }
    let version = line(&mut at)?;
    let rev = line(&mut at)?;
    let want_version = format!("verus_builtin_macros {VERUS_BUILTIN_MACROS_VERSION}");
    let want_rev = format!("verus_git_rev {VERUS_GIT_REV}");
    if version != want_version || rev != want_rev {
        return Err(format!(
            "the helper vendors a different Verus erasure (`{version}`, `{rev}`) than this \
             transpiler expects (`{want_version}`, `{want_rev}`)"
        ));
    }
    if line(&mut at)? != format!("blocks {expected}") {
        return Err(format!("expected `blocks {expected}`"));
    }
    let mut results = Vec::with_capacity(expected);
    for _ in 0..expected {
        let header = line(&mut at)?;
        let (tag, len) = header
            .split_once(' ')
            .and_then(|(tag, len)| Some((tag, len.parse::<usize>().ok()?)))
            .ok_or_else(|| format!("malformed result header `{header}`"))?;
        let rest = &response[at..];
        if rest.len() < len + 1 || rest[len] != b'\n' {
            return Err("truncated result payload".to_string());
        }
        let payload = String::from_utf8(rest[..len].to_vec())
            .map_err(|_| "result payload is not UTF-8".to_string())?;
        at += len + 1;
        results.push(match tag {
            "ok" => Ok(payload),
            "err" => Err(payload),
            _ => return Err(format!("unknown result tag `{tag}`")),
        });
    }
    if at != response.len() {
        return Err("trailing bytes after the last result".to_string());
    }
    Ok(results)
}

/// Erase `blocks` (each the token text of one `verus!` body) in one helper
/// process.
fn run_helper(
    config: &VerusExecConfig,
    blocks: &[String],
) -> Result<Vec<Result<String, String>>, String> {
    let helper = helper_path(config)?;
    let failure = |what: String| format!("--verus-exec: helper `{}` {what}", helper.display());
    let mut child = Command::new(&helper)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| failure(format!("could not be started: {error}")))?;
    let request = encode_request(blocks);
    let mut stdin = child.stdin.take().expect("helper stdin is piped");
    // Write from another thread so a large response cannot block the helper
    // while this thread is still writing the request.
    let writer = std::thread::spawn(move || stdin.write_all(&request));
    let output = child
        .wait_with_output()
        .map_err(|error| failure(format!("could not be waited for: {error}")))?;
    let written = writer.join().expect("helper stdin writer does not panic");
    if !output.status.success() {
        return Err(failure(format!(
            "failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    written.map_err(|error| failure(format!("did not accept the request: {error}")))?;
    decode_response(&output.stdout, blocks.len())
        .map_err(|error| failure(format!("returned an unusable response: {error}")))
}

// ---------------------------------------------------------------------------
// verus_keep_ghost / verus_keep_ghost_body evaluation

/// Result of partially evaluating a cfg predicate with the Verus driver cfgs
/// fixed to false.
enum Partial {
    True,
    False,
    /// The predicate still depends on other cfgs; this is its simplified form,
    /// free of Verus driver cfgs.
    Residual(Meta),
}

fn meta_mentions_driver_cfg(tokens: TokenStream) -> bool {
    tokens.into_iter().any(|tree| match tree {
        TokenTree::Ident(ident) => VERUS_DRIVER_CFGS.iter().any(|name| ident == name),
        TokenTree::Group(group) => meta_mentions_driver_cfg(group.stream()),
        _ => false,
    })
}

fn cfg_args(list: &syn::MetaList) -> Result<Vec<Meta>, String> {
    list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .map(|args| args.into_iter().collect())
        .map_err(|error| format!("malformed cfg predicate `{}`: {error}", quote::quote!(#list)))
}

fn partial_eval(meta: &Meta) -> Result<Partial, String> {
    match meta {
        Meta::Path(path)
            if VERUS_DRIVER_CFGS
                .iter()
                .any(|name| path.is_ident(name)) =>
        {
            Ok(Partial::False)
        }
        Meta::Path(_) | Meta::NameValue(_) => Ok(Partial::Residual(meta.clone())),
        Meta::List(list) if list.path.is_ident("all") || list.path.is_ident("any") => {
            let is_all = list.path.is_ident("all");
            let mut residuals = Vec::new();
            for arg in cfg_args(list)? {
                match partial_eval(&arg)? {
                    Partial::True if is_all => {}
                    Partial::False if !is_all => {}
                    Partial::True => return Ok(Partial::True),
                    Partial::False => return Ok(Partial::False),
                    Partial::Residual(residual) => residuals.push(residual),
                }
            }
            Ok(match residuals.len() {
                0 if is_all => Partial::True,
                0 => Partial::False,
                1 => Partial::Residual(residuals.pop().unwrap()),
                _ => {
                    let path = &list.path;
                    Partial::Residual(syn::parse_quote!(#path(#(#residuals),*)))
                }
            })
        }
        Meta::List(list) if list.path.is_ident("not") => {
            let mut args = cfg_args(list)?;
            if args.len() != 1 {
                return Err(format!(
                    "malformed cfg predicate `{}`: `not` takes one argument",
                    quote::quote!(#list)
                ));
            }
            Ok(match partial_eval(&args.remove(0))? {
                Partial::True => Partial::False,
                Partial::False => Partial::True,
                Partial::Residual(residual) => Partial::Residual(syn::parse_quote!(not(#residual))),
            })
        }
        Meta::List(_) => Ok(Partial::Residual(meta.clone())),
    }
}

/// What to do with the node that owns an attribute list.
enum AttrsVerdict {
    Keep,
    /// A `#[cfg]` on the node evaluated to false: drop the node.
    Remove,
}

/// Rewrite one attribute list: evaluate `cfg` / `cfg_attr` predicates that
/// mention a Verus driver cfg, leaving every other attribute untouched.
fn process_attrs(attrs: &mut Vec<Attribute>, changed: &mut bool) -> Result<AttrsVerdict, String> {
    let mut output = Vec::with_capacity(attrs.len());
    let mut verdict = AttrsVerdict::Keep;
    for attribute in attrs.drain(..) {
        let Meta::List(list) = &attribute.meta else {
            output.push(attribute);
            continue;
        };
        let is_cfg = list.path.is_ident("cfg");
        let is_cfg_attr = list.path.is_ident("cfg_attr");
        if !(is_cfg || is_cfg_attr) || !meta_mentions_driver_cfg(list.tokens.clone()) {
            output.push(attribute);
            continue;
        }
        *changed = true;
        if is_cfg {
            let predicate = list
                .parse_args::<Meta>()
                .map_err(|error| format!("malformed `#[cfg(..)]`: {error}"))?;
            match partial_eval(&predicate)? {
                Partial::True => {}
                Partial::False => verdict = AttrsVerdict::Remove,
                Partial::Residual(residual) => {
                    let mut kept = attribute.clone();
                    kept.meta = syn::parse_quote!(cfg(#residual));
                    output.push(kept);
                }
            }
            continue;
        }
        // cfg_attr(predicate, attr1, attr2, ...)
        let mut args = list
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .map_err(|error| format!("malformed `#[cfg_attr(..)]`: {error}"))?
            .into_iter();
        let Some(predicate) = args.next() else {
            return Err("malformed `#[cfg_attr(..)]`: missing predicate".to_string());
        };
        let rest = args.collect::<Vec<_>>();
        match partial_eval(&predicate)? {
            Partial::False => {}
            Partial::True => {
                // Rustc expands a true cfg_attr into its attributes, which
                // may themselves be cfgs; re-process them in place.
                let mut expanded = rest
                    .into_iter()
                    .map(|meta| Attribute {
                        pound_token: attribute.pound_token,
                        style: attribute.style,
                        bracket_token: attribute.bracket_token,
                        meta,
                    })
                    .collect::<Vec<_>>();
                if let AttrsVerdict::Remove = process_attrs(&mut expanded, changed)? {
                    verdict = AttrsVerdict::Remove;
                }
                output.extend(expanded);
            }
            Partial::Residual(residual) => {
                let mut kept = attribute.clone();
                kept.meta = syn::parse_quote!(cfg_attr(#residual, #(#rest),*));
                output.push(kept);
            }
        }
    }
    *attrs = output;
    Ok(verdict)
}

/// Evaluates Verus driver cfgs on the attribute owners rustc's cfg-stripping
/// visits and this pass supports. Anything left over is caught by
/// `ResidueAudit`.
#[derive(Default)]
struct DriverCfgPass {
    changed: bool,
    error: Option<String>,
}

impl DriverCfgPass {
    fn attrs(&mut self, attrs: &mut Vec<Attribute>) -> bool {
        if self.error.is_some() {
            return true;
        }
        match process_attrs(attrs, &mut self.changed) {
            Ok(AttrsVerdict::Keep) => true,
            Ok(AttrsVerdict::Remove) => false,
            Err(error) => {
                self.error = Some(format!("--verus-exec: {error}"));
                true
            }
        }
    }

    fn retain<T>(&mut self, nodes: &mut Vec<T>, attrs_of: impl Fn(&mut T) -> Option<&mut Vec<Attribute>>) {
        let mut kept = Vec::with_capacity(nodes.len());
        for mut node in nodes.drain(..) {
            let keep = match attrs_of(&mut node) {
                Some(attrs) => self.attrs(attrs),
                None => true,
            };
            if keep {
                kept.push(node);
            }
        }
        *nodes = kept;
    }

    fn retain_punctuated<T, P: Default>(
        &mut self,
        nodes: &mut Punctuated<T, P>,
        attrs_of: impl Fn(&mut T) -> &mut Vec<Attribute>,
    ) {
        let mut kept = Punctuated::new();
        for mut node in std::mem::take(nodes).into_iter() {
            if self.attrs(attrs_of(&mut node)) {
                kept.push(node);
            }
        }
        *nodes = kept;
    }
}

fn item_attrs(item: &mut Item) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        Item::Const(item) => &mut item.attrs,
        Item::Enum(item) => &mut item.attrs,
        Item::ExternCrate(item) => &mut item.attrs,
        Item::Fn(item) => &mut item.attrs,
        Item::ForeignMod(item) => &mut item.attrs,
        Item::Impl(item) => &mut item.attrs,
        Item::Macro(item) => &mut item.attrs,
        Item::Mod(item) => &mut item.attrs,
        Item::Static(item) => &mut item.attrs,
        Item::Struct(item) => &mut item.attrs,
        Item::Trait(item) => &mut item.attrs,
        Item::TraitAlias(item) => &mut item.attrs,
        Item::Type(item) => &mut item.attrs,
        Item::Union(item) => &mut item.attrs,
        Item::Use(item) => &mut item.attrs,
        _ => return None,
    })
}

fn impl_item_attrs(item: &mut syn::ImplItem) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        syn::ImplItem::Const(item) => &mut item.attrs,
        syn::ImplItem::Fn(item) => &mut item.attrs,
        syn::ImplItem::Type(item) => &mut item.attrs,
        syn::ImplItem::Macro(item) => &mut item.attrs,
        _ => return None,
    })
}

fn trait_item_attrs(item: &mut syn::TraitItem) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        syn::TraitItem::Const(item) => &mut item.attrs,
        syn::TraitItem::Fn(item) => &mut item.attrs,
        syn::TraitItem::Type(item) => &mut item.attrs,
        syn::TraitItem::Macro(item) => &mut item.attrs,
        _ => return None,
    })
}

fn foreign_item_attrs(item: &mut syn::ForeignItem) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        syn::ForeignItem::Fn(item) => &mut item.attrs,
        syn::ForeignItem::Static(item) => &mut item.attrs,
        syn::ForeignItem::Type(item) => &mut item.attrs,
        syn::ForeignItem::Macro(item) => &mut item.attrs,
        _ => return None,
    })
}

fn stmt_attrs(stmt: &mut syn::Stmt) -> Option<&mut Vec<Attribute>> {
    match stmt {
        syn::Stmt::Local(local) => Some(&mut local.attrs),
        syn::Stmt::Item(item) => item_attrs(item),
        syn::Stmt::Macro(mac) => Some(&mut mac.attrs),
        syn::Stmt::Expr(..) => None,
    }
}

fn fn_arg_attrs(arg: &mut syn::FnArg) -> &mut Vec<Attribute> {
    match arg {
        syn::FnArg::Receiver(receiver) => &mut receiver.attrs,
        syn::FnArg::Typed(typed) => &mut typed.attrs,
    }
}

impl VisitMut for DriverCfgPass {
    fn visit_file_mut(&mut self, file: &mut syn::File) {
        if !self.attrs(&mut file.attrs) {
            self.error.get_or_insert_with(|| {
                "--verus-exec: a file-level `#![cfg(..)]` on a Verus driver cfg removes the whole module; unsupported"
                    .to_string()
            });
        }
        self.retain(&mut file.items, item_attrs);
        syn::visit_mut::visit_file_mut(self, file);
    }

    fn visit_item_mod_mut(&mut self, module: &mut syn::ItemMod) {
        if let Some((_, items)) = &mut module.content {
            self.retain(items, item_attrs);
        }
        syn::visit_mut::visit_item_mod_mut(self, module);
    }

    fn visit_item_impl_mut(&mut self, item: &mut syn::ItemImpl) {
        self.retain(&mut item.items, impl_item_attrs);
        syn::visit_mut::visit_item_impl_mut(self, item);
    }

    fn visit_item_trait_mut(&mut self, item: &mut syn::ItemTrait) {
        self.retain(&mut item.items, trait_item_attrs);
        syn::visit_mut::visit_item_trait_mut(self, item);
    }

    fn visit_item_foreign_mod_mut(&mut self, item: &mut syn::ItemForeignMod) {
        self.retain(&mut item.items, foreign_item_attrs);
        syn::visit_mut::visit_item_foreign_mod_mut(self, item);
    }

    fn visit_item_enum_mut(&mut self, item: &mut syn::ItemEnum) {
        self.retain_punctuated(&mut item.variants, |variant| &mut variant.attrs);
        syn::visit_mut::visit_item_enum_mut(self, item);
    }

    fn visit_fields_named_mut(&mut self, fields: &mut syn::FieldsNamed) {
        self.retain_punctuated(&mut fields.named, |field| &mut field.attrs);
        syn::visit_mut::visit_fields_named_mut(self, fields);
    }

    fn visit_fields_unnamed_mut(&mut self, fields: &mut syn::FieldsUnnamed) {
        self.retain_punctuated(&mut fields.unnamed, |field| &mut field.attrs);
        syn::visit_mut::visit_fields_unnamed_mut(self, fields);
    }

    fn visit_signature_mut(&mut self, signature: &mut syn::Signature) {
        self.retain_punctuated(&mut signature.inputs, fn_arg_attrs);
        syn::visit_mut::visit_signature_mut(self, signature);
    }

    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        self.retain(&mut block.stmts, stmt_attrs);
        syn::visit_mut::visit_block_mut(self, block);
    }

    fn visit_expr_match_mut(&mut self, expr: &mut syn::ExprMatch) {
        self.retain(&mut expr.arms, |arm| Some(&mut arm.attrs));
        syn::visit_mut::visit_expr_match_mut(self, expr);
    }

    fn visit_expr_struct_mut(&mut self, expr: &mut syn::ExprStruct) {
        self.retain_punctuated(&mut expr.fields, |field| &mut field.attrs);
        syn::visit_mut::visit_expr_struct_mut(self, expr);
    }
}

// ---------------------------------------------------------------------------
// fail-closed audit of what survives

/// Macro names exported by `verus_builtin_macros` and
/// `verus_state_machines_macros` (and vstd's `macro_rules!` front ends for
/// them). None of them may survive into the transpiled program.
const VERUS_MACROS: &[&str] = &[
    "verus",
    "verus_keep_ghost",
    "verus_erase_ghost",
    "verus_impl",
    "verus_trait_impl",
    "verus_proof_expr",
    "verus_exec_expr",
    "verus_exec_expr_keep_ghost",
    "verus_exec_expr_erase_ghost",
    "verus_proof_macro_exprs",
    "verus_exec_macro_exprs",
    "verus_exec_inv_macro_exprs",
    "verus_ghost_inv_macro_exprs",
    "verus_proof_macro_explicit_exprs",
    "struct_with_invariants",
    "atomic_with_ghost",
    "atomic_with_ghost_helper",
    "calc",
    "calc_proc_macro",
    "proof",
    "proof_decl",
    "proof_with",
    "fndecl",
    "exec_spec",
    "tokenized_state_machine",
    "state_machine",
    "case_on_next",
    "case_on_next_strong",
    "case_on_init",
    "open_atomic_invariant",
    "open_local_invariant",
    "open_atomic_invariant_in_proof",
    "open_local_invariant_in_proof",
];

/// Crates whose macros are Verus-only, whatever their name.
const VERUS_MACRO_CRATES: &[&str] = &[
    "vstd",
    "verus_builtin",
    "verus_builtin_macros",
    "builtin_macros",
    "verus_state_machines_macros",
    "state_machines_macros",
];

fn verus_macro_name(path: &syn::Path) -> Option<String> {
    let first = path.segments.first()?.ident.to_string();
    let last = path.segments.last()?.ident.to_string();
    if path.segments.len() > 1 && VERUS_MACRO_CRATES.contains(&first.as_str()) {
        return Some(quote::quote!(#path).to_string().replace(' ', ""));
    }
    (path.segments.len() == 1 && VERUS_MACROS.contains(&last.as_str())).then_some(last)
}

/// The first `<verus macro>!` invocation inside a token stream, if any.
fn tokens_invoke_verus_macro(tokens: TokenStream) -> Option<String> {
    let trees = tokens.into_iter().collect::<Vec<_>>();
    for (index, tree) in trees.iter().enumerate() {
        match tree {
            TokenTree::Ident(ident)
                if VERUS_MACROS.iter().any(|name| ident == name)
                    && matches!(trees.get(index + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!') =>
            {
                return Some(ident.to_string());
            }
            TokenTree::Group(group) => {
                if let Some(found) = tokens_invoke_verus_macro(group.stream()) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Default)]
struct ResidueAudit {
    errors: Vec<String>,
}

impl<'ast> Visit<'ast> for ResidueAudit {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(name) = verus_macro_name(&mac.path) {
            self.errors.push(format!(
                "--verus-exec: `{name}!` is a Verus macro this pass does not erase (only item-level `verus! {{ }}` is supported)"
            ));
        } else if let Some(inner) = tokens_invoke_verus_macro(mac.tokens.clone()) {
            // e.g. Lion's `macro_rules! reactor_log_action { .. => { verus! {
            // impl Reactor { .. } } } }`: rustc erases what the expansion
            // produces, but this pass only sees the unexpanded tokens.
            let path = &mac.path;
            self.errors.push(format!(
                "--verus-exec: `{}!` carries a `{inner}!` invocation in its tokens; macro-generated Verus code is not erased by this pass",
                quote::quote!(#path).to_string().replace(' ', "")
            ));
        } else if meta_mentions_driver_cfg(mac.tokens.clone()) {
            let path = &mac.path;
            self.errors.push(format!(
                "--verus-exec: `{}!` mentions a Verus driver cfg inside its tokens, which this pass cannot evaluate",
                quote::quote!(#path).to_string().replace(' ', "")
            ));
        }
        syn::visit::visit_macro(self, mac);
    }

    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        if let Meta::List(list) = &attribute.meta
            && (list.path.is_ident("cfg") || list.path.is_ident("cfg_attr"))
            && meta_mentions_driver_cfg(list.tokens.clone())
        {
            self.errors.push(format!(
                "--verus-exec: `{}` survives in a position this pass does not evaluate",
                quote::quote!(#attribute)
            ));
        }
        syn::visit::visit_attribute(self, attribute);
    }
}

/// Test support: the helper executable, built once per test process.
#[cfg(test)]
pub(crate) mod test_helper {
    use std::path::PathBuf;
    use std::sync::OnceLock;

    /// `$RUSTY_CPP_VERUS_ERASE` when set; otherwise `cargo build` of the
    /// helper into its own target directory under the workspace's `target/`
    /// (a separate directory, so the build neither waits on the lock of the
    /// `cargo test` running this binary nor unifies features with it).
    pub(crate) fn path() -> PathBuf {
        static HELPER: OnceLock<PathBuf> = OnceLock::new();
        HELPER
            .get_or_init(|| {
                if let Some(path) = std::env::var_os(super::HELPER_ENV).filter(|v| !v.is_empty()) {
                    return PathBuf::from(path);
                }
                let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .expect("transpiler/ has a parent")
                    .to_path_buf();
                let target_dir = workspace.join("target").join("verus-erase-test-helper");
                let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
                let status = std::process::Command::new(cargo)
                    .current_dir(&workspace)
                    .args([
                        "build",
                        "--release",
                        "--locked",
                        "-p",
                        "verus-erase",
                        "--bin",
                        super::HELPER_NAME,
                        "--target-dir",
                    ])
                    .arg(&target_dir)
                    .status()
                    .expect("cargo runs");
                assert!(status.success(), "building the verus-erase helper failed");
                target_dir
                    .join("release")
                    .join(format!("{}{}", super::HELPER_NAME, std::env::consts::EXE_SUFFIX))
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> VerusExecConfig {
        VerusExecConfig {
            enabled: true,
            dump_dir: None,
            helper: Some(test_helper::path()),
        }
    }

    fn erase_source(source: &str) -> Result<Cow<'_, str>, String> {
        super::erase_source(&config(), source)
    }

    fn erase(source: &str) -> String {
        erase_source(source).expect("erasure succeeds").into_owned()
    }

    fn pretty(source: &str) -> String {
        prettyplease::unparse(&syn::parse_file(source).expect("expected source parses"))
    }

    #[test]
    fn source_without_verus_constructs_is_returned_byte_for_byte() {
        let source = "// a comment the pre-pass must not touch\nfn  main( ) { }\n";
        assert!(matches!(erase_source(source).unwrap(), Cow::Borrowed(_)));
        let mentions = "// verus is only mentioned here\nfn  main( ) { }\n";
        assert!(matches!(erase_source(mentions).unwrap(), Cow::Borrowed(_)));
        let lookalike = "mod proof_utils { pub fn calc() {} }\n// vstd\n";
        assert!(matches!(erase_source(lookalike).unwrap(), Cow::Borrowed(_)));
        assert_eq!(
            prepare_source(&VerusExecConfig::default(), "verus! { spec fn f() {} }".to_string())
                .unwrap(),
            "verus! { spec fn f() {} }",
            "disabled mode is the identity"
        );
    }

    #[test]
    fn verus_block_erases_to_executable_code_only() {
        let erased = erase(
            r#"
use vstd::prelude::*;
verus! {
pub struct Counter { pub value: u64 }

spec fn limit() -> nat { 100 }

proof fn limit_is_positive() ensures limit() > 0 {}

impl Counter {
    pub open spec fn wf(&self) -> bool { self.value <= limit() }

    pub fn bump(&mut self) -> (r: u64)
        requires old(self).wf(), old(self).value < 100,
        ensures self.wf(), r == self.value,
    {
        let ghost before = self.value;
        proof { assert(before < 100); }
        self.value = self.value + 1;
        assert(self.value == before + 1);
        self.value
    }
}
}
"#,
        );
        assert_eq!(
            erased,
            pretty(
                r#"
use vstd::prelude::*;
pub struct Counter { pub value: u64 }
impl Counter {
    pub fn bump(&mut self) -> u64 {
        {}
        self.value = self.value + 1;
        {};
        self.value
    }
}
"#
            )
        );
    }

    #[test]
    fn verus_blocks_inside_inline_modules_are_erased() {
        let erased = erase(
            r#"
pub mod outer {
    pub mod inner {
        verus! {
            pub fn f(x: u8) -> u8 ensures x == x { x }
            spec fn g() -> int { 0 }
        }
    }
    vstd::prelude::verus! { pub const K: u8 = 1; }
}
"#,
        );
        assert_eq!(
            erased,
            pretty("pub mod outer { pub mod inner { pub fn f(x: u8) -> u8 { x } } pub const K: u8 = 1; }")
        );
    }

    #[test]
    fn verus_keep_ghost_cfgs_are_false() {
        let erased = erase(
            r#"
#![cfg_attr(verus_keep_ghost, verus::trusted)]
#![allow(dead_code)]
#[cfg(verus_keep_ghost)]
use vstd::invariant::*;
#[cfg(not(verus_keep_ghost))]
pub fn plain() {}
#[cfg(all(feature = "x", not(verus_keep_ghost_body)))]
pub fn featured() {}
#[cfg(any(verus_keep_ghost, test))]
pub fn test_only() {}
#[cfg_attr(not(verus_keep_ghost), inline)]
pub fn inlined() {}
pub struct S {
    #[cfg(verus_keep_ghost)]
    ghost: u8,
    kept: u8,
}
"#,
        );
        assert_eq!(
            erased,
            pretty(
                r#"
#![allow(dead_code)]
pub fn plain() {}
#[cfg(feature = "x")]
pub fn featured() {}
#[cfg(test)]
pub fn test_only() {}
#[inline]
pub fn inlined() {}
pub struct S {
    kept: u8,
}
"#
            )
        );
    }

    #[test]
    fn verus_keep_ghost_items_inside_verus_blocks_are_removed() {
        let erased = erase(
            "verus! {\n#[cfg(verus_keep_ghost)]\npub fn ghost_only() {}\npub fn kept() {}\n}\n",
        );
        assert_eq!(erased, pretty("pub fn kept() {}"));
    }

    #[test]
    fn other_verus_macros_fail_closed() {
        for source in [
            "verus_impl! { fn f() {} }",
            "verus_keep_ghost! { fn f() {} }",
            "fn f() { proof! { assert(true); } }",
            "fn f() { verus! { fn g() {} } }",
            "struct_with_invariants! { struct S {} }",
            "vstd::atomic_with_ghost!(x => load(); ghost g => {});",
        ] {
            let error = erase_source(source).expect_err(source);
            assert!(error.contains("Verus macro"), "{source}: {error}");
        }
    }

    #[test]
    fn macro_generated_verus_blocks_fail_closed() {
        // Lion's reactor/executor `*_log_action` shape.
        let error = erase_source(
            "macro_rules! log_action { ($name:ident) => { verus! { impl R { fn $name(&self) {} } } }; }",
        )
        .unwrap_err();
        assert!(error.contains("macro_rules") && error.contains("verus!"), "{error}");
    }

    #[test]
    fn driver_cfg_inside_macro_tokens_fails_closed() {
        let error = erase_source(
            "thread_local! { #[cfg(verus_keep_ghost)] static X: u8 = 0; }",
        )
        .unwrap_err();
        assert!(error.contains("thread_local"), "{error}");
    }

    #[test]
    fn verus_syntax_errors_fail_closed() {
        let error = erase_source("verus! { fn f() -> {} }").unwrap_err();
        assert!(error.contains("Verus rejected"), "{error}");
    }

    #[test]
    fn dump_writes_the_prepared_unit_under_the_crate_name() {
        let dir = tempfile::tempdir().unwrap();
        let config = VerusExecConfig {
            enabled: true,
            dump_dir: Some(dir.path().to_path_buf()),
            helper: Some(test_helper::path()),
        };
        let prepared = prepare_source(&config, "verus! { fn f() ensures true {} }".to_string())
            .unwrap();
        dump_source(&config, "demo", Path::new("src/lib.rs"), &prepared).unwrap();
        let dumped = std::fs::read_to_string(dir.path().join("demo/src/lib.rs")).unwrap();
        assert_eq!(dumped, pretty("fn f() {}"));
    }

    #[test]
    fn one_helper_request_carries_every_block_of_a_file() {
        // Two blocks plus one inside an inline module: all three are in the
        // prefetch batch, so erasing them needs no further request.
        let file = syn::parse_file(
            "verus! { fn a() {} } mod m { verus! { fn b() ensures true {} } } verus! { spec fn s() -> int { 0 } }",
        )
        .unwrap();
        let config = config();
        let mut eraser = Eraser::prefetch(&config, &file).unwrap();
        assert_eq!(eraser.ready.values().map(VecDeque::len).sum::<usize>(), 3);
        let erased = erase(
            "verus! { fn a() {} } mod m { verus! { fn b() ensures true {} } } verus! { spec fn s() -> int { 0 } }",
        );
        assert_eq!(erased, pretty("fn a() {} mod m { fn b() {} }"));
        // The prefetched results serve the erasure without a second request.
        for body in ["fn a() {}", "fn b() ensures true {}", "spec fn s() -> int { 0 }"] {
            let tokens = TokenStream::from_str(body).unwrap();
            assert!(eraser.ready.contains_key(&tokens.to_string()), "{body}");
            assert!(eraser.erase(&tokens).unwrap().is_ok(), "{body}");
        }
        assert!(eraser.ready.values().all(VecDeque::is_empty));
    }

    #[test]
    fn helper_must_vendor_the_expected_verus_revision() {
        let good = format!(
            "{HELPER_PROTOCOL}\nverus_builtin_macros {VERUS_BUILTIN_MACROS_VERSION}\nverus_git_rev {VERUS_GIT_REV}\nblocks 1\nok 2\nab\n"
        );
        assert_eq!(decode_response(good.as_bytes(), 1).unwrap(), vec![Ok("ab".to_string())]);
        let other_rev = good.replace(VERUS_GIT_REV, "0000000000000000000000000000000000000000");
        let error = decode_response(other_rev.as_bytes(), 1).unwrap_err();
        assert!(error.contains("different Verus erasure"), "{error}");
        assert!(decode_response(good.as_bytes(), 2).is_err());
        assert!(decode_response(&good.as_bytes()[..good.len() - 1], 1).is_err());
    }

    #[test]
    fn missing_helper_fails_closed_with_build_instructions() {
        let config = VerusExecConfig {
            enabled: true,
            dump_dir: None,
            helper: Some(PathBuf::from("/nonexistent/rusty-cpp-verus-erase")),
        };
        let error = super::erase_source(&config, "verus! { fn f() {} }").unwrap_err();
        assert!(error.contains("could not be started"), "{error}");
        // A file without Verus constructs never needs the helper.
        assert!(matches!(
            super::erase_source(&config, "fn f() {}").unwrap(),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn transpiler_and_helper_crate_agree_on_the_vendored_revision() {
        let lib = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../verus-erase/src/lib.rs"),
        )
        .unwrap();
        assert!(lib.contains(&format!(
            "pub const VERUS_BUILTIN_MACROS_VERSION: &str = \"{VERUS_BUILTIN_MACROS_VERSION}\";"
        )));
        assert!(lib.contains(&format!("pub const VERUS_GIT_REV: &str = \"{VERUS_GIT_REV}\";")));
        let protocol = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../verus-erase/src/protocol.rs"),
        )
        .unwrap();
        assert!(protocol.contains(&format!("pub const PROTOCOL: &str = \"{HELPER_PROTOCOL}\";")));
        let output = Command::new(test_helper::path()).arg("--version").output().unwrap();
        assert!(output.status.success());
        let version = String::from_utf8(output.stdout).unwrap();
        assert!(version.contains(VERUS_GIT_REV) && version.contains(VERUS_BUILTIN_MACROS_VERSION), "{version}");
    }
}
