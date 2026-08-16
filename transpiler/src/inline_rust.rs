use crate::transpile;
use crate::types::UserTypeMap;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const IF_RUSTYCPP_RUST: &str = "#if RUSTYCPP_RUST";
const ELSE_DIRECTIVE: &str = "#else";
const ENDIF_DIRECTIVE: &str = "#endif";
const RUST_BEGIN_PREFIX: &str = "/*RUSTYCPP:RUST-BEGIN";
const RUST_END_PREFIX: &str = "/*RUSTYCPP:RUST-END";
const LEGACY_AT_RUST_PREFIX: &str = "@rust";
const GEN_BEGIN_PREFIX: &str = "/*RUSTYCPP:GEN-BEGIN ";
const GEN_END_PREFIX: &str = "/*RUSTYCPP:GEN-END id=";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineRustMode {
    Check,
    Rewrite,
    EmitRust {
        output: PathBuf,
        block_ids: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct InlineRustOptions {
    pub mode: InlineRustMode,
    pub files: Vec<PathBuf>,
}

struct LoadedCarrier {
    path: PathBuf,
    content: String,
    blocks: Vec<ParsedBlock>,
    cpp_abi: Option<crate::cpp_abi::CppAbiInlineCarrierPlan>,
}

#[derive(Clone, Debug)]
struct InlineRustContext {
    authenticated_cpp_inherit_roots: HashSet<String>,
    authenticated_sysroot_roots: HashSet<String>,
    import_bindings: transpile::RustItemImportBindings,
}

impl Default for InlineRustContext {
    fn default() -> Self {
        Self {
            authenticated_cpp_inherit_roots: HashSet::new(),
            authenticated_sysroot_roots: HashSet::from([
                "std".to_string(),
                "core".to_string(),
            ]),
            import_bindings: transpile::RustItemImportBindings::new(),
        }
    }
}

fn inline_rust_context(path: &Path, blocks: &[ParsedBlock]) -> Result<InlineRustContext, String> {
    fn items_request_cpp_inherit(items: &[syn::Item]) -> bool {
        items.iter().any(|item| match item {
            syn::Item::Impl(item_impl) => item_impl.attrs.iter().any(|attribute| {
                attribute
                    .path()
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "cpp_inherit")
            }),
            syn::Item::Mod(item_mod) => item_mod
                .content
                .as_ref()
                .is_some_and(|(_, nested)| items_request_cpp_inherit(nested)),
            _ => false,
        })
    }

    let mut import_bindings = transpile::RustItemImportBindings::new();
    let mut requests_cpp_inherit = false;
    for block in blocks {
        let file = syn::parse_file(&block.rust_payload_normalized).map_err(|error| {
            format!(
                "{}:{}: failed to parse inline block id={}: {}",
                path.display(), block.if_line, block.id, error
            )
        })?;
        requests_cpp_inherit |= items_request_cpp_inherit(&file.items);
        for (key, targets) in transpile::collect_rust_item_import_bindings(&file.items) {
            import_bindings.entry(key).or_default().extend(targets);
        }
    }
    let Some(manifest) = crate::nearest_cargo_manifest(path) else {
        if requests_cpp_inherit {
            return Err(format!(
                "{}: inline `cpp_inherit` requires a Cargo manifest so the `rusty` provider can be authenticated",
                path.display()
            ));
        }
        return Ok(InlineRustContext {
            import_bindings,
            ..InlineRustContext::default()
        });
    };
    // Inline carriers are source-only: there is no Cargo build invocation
    // whose feature set can be inferred.  Authenticate against the
    // conservative all-feature graph so optional reserved-name providers fail
    // closed rather than inheriting Cargo's unrelated default graph. A C++
    // carrier is not a Cargo target root, so dependency-kind provenance is
    // also deliberately conservative (normal + dev + build).
    let conservative_cargo_flags = ["--all-features".to_string()];
    let compilation = crate::metadata::CargoCompilationContext::conservative();
    Ok(InlineRustContext {
        authenticated_cpp_inherit_roots: if requests_cpp_inherit {
            crate::authenticated_cpp_inherit_roots_for_compilation(
                &manifest,
                None,
                &conservative_cargo_flags,
                &compilation,
            )
            .map_err(|error| {
                format!(
                    "{}: could not authenticate inline compiler markers from {}: {error}",
                    path.display(), manifest.display()
                )
            })?
        } else {
            HashSet::new()
        },
        authenticated_sysroot_roots: crate::authenticated_sysroot_roots_for_compilation(
            &manifest,
            None,
            &conservative_cargo_flags,
            &compilation,
        )
        .map_err(|error| {
                format!(
                    "{}: could not authenticate inline sysroot crates from {}: {error}",
                    path.display(), manifest.display()
                )
            })?,
        import_bindings,
    })
}

pub fn run_inline_rust(options: &InlineRustOptions) -> Result<(), String> {
    if options.files.is_empty() {
        return Err("inline-rust: at least one path is required".to_string());
    }

    if let InlineRustMode::EmitRust { .. } = &options.mode {
        if options.files.len() != 1 {
            return Err(format!(
                "inline-rust --emit-rust requires exactly one --files input; got {}",
                options.files.len()
            ));
        }
    }

    let mut carriers = Vec::with_capacity(options.files.len());
    for path in &options.files {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("{}: failed to read file: {}", path.display(), e))?;
        let blocks = parse_blocks(path, &content)?;
        // Authenticate before any carrier is prepared or written so the
        // multi-file operation remains all-or-nothing.
        let _ = inline_rust_context(path, &blocks)?;
        carriers.push(LoadedCarrier {
            path: path.clone(),
            content,
            blocks,
            cpp_abi: None,
        });
    }

    prepare_cpp_abi_carriers(&mut carriers)?;

    match &options.mode {
        InlineRustMode::Check => check_carriers(&carriers),
        InlineRustMode::Rewrite => rewrite_carriers(&carriers),
        InlineRustMode::EmitRust { output, block_ids } => {
            emit_rust_carrier(&carriers[0], output, block_ids)
        }
    }
}

fn parse_block_files(carrier: &LoadedCarrier) -> Result<Vec<syn::File>, String> {
    carrier
        .blocks
        .iter()
        .map(|block| {
            syn::parse_file(&block.rust_payload_normalized).map_err(|error| {
                format!(
                    "{}:{}: failed to parse inline block id={}: {}",
                    carrier.path.display(),
                    block.if_line,
                    block.id,
                    error
                )
            })
        })
        .collect()
}

fn prepare_cpp_abi_carriers(carriers: &mut [LoadedCarrier]) -> Result<(), String> {
    let any_marker = carriers.iter().flat_map(|carrier| &carrier.blocks).any(|block| {
        crate::cpp_abi::source_mentions_reserved_marker(&block.rust_payload_normalized)
    });
    if !any_marker {
        return Ok(());
    }

    let files = carriers
        .iter()
        .map(parse_block_files)
        .collect::<Result<Vec<_>, _>>()?;
    let sources = carriers
        .iter()
        .map(|carrier| carrier.path.display().to_string())
        .collect::<Vec<_>>();
    crate::cpp_abi::validate_inline_projected_cpp_name_collisions(&sources, &files)?;
    let external_contracts = crate::cpp_abi::inline_external_contract_indexes(&files)?;
    let names = files
        .iter()
        .map(|files| crate::cpp_abi::inline_contract_names(files))
        .collect::<Result<Vec<_>, _>>()?;
    let identities = carriers
        .iter()
        .enumerate()
        .map(|(index, carrier)| {
            if names[index].is_empty() {
                Ok("unused".to_string())
            } else {
                cpp_abi_carrier_identity(carrier)
            }
        })
        .collect::<Result<Vec<_>, String>>()?;
    let helper_names = files
        .iter()
        .zip(&identities)
        .map(|(files, identity)| {
            crate::cpp_abi::inline_generated_helper_names(files, identity)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut owner_by_name = BTreeMap::<String, usize>::new();
    for (carrier_index, carrier_names) in names.iter().enumerate() {
        for name in carrier_names {
            if let Some(previous) = owner_by_name.insert(name.clone(), carrier_index)
                && previous != carrier_index
            {
                return Err(format!(
                    "inline-rust cpp_abi name `{name}` is provided by more than one carrier: {} and {}",
                    carriers[previous].path.display(),
                    carriers[carrier_index].path.display()
                ));
            }
        }
    }
    let mut owner_by_helper = BTreeMap::<String, usize>::new();
    for (carrier_index, carrier_helpers) in helper_names.iter().enumerate() {
        for helper in carrier_helpers {
            if let Some(previous) = owner_by_helper.insert(helper.clone(), carrier_index)
                && previous != carrier_index
            {
                return Err(format!(
                    "inline-rust cpp_abi generated helper `{helper}` collides across carriers {} and {}",
                    carriers[previous].path.display(),
                    carriers[carrier_index].path.display()
                ));
            }
        }
    }

    for index in 0..carriers.len() {
        let plan = crate::cpp_abi::prepare_inline_carrier(
            &files[index],
            &external_contracts[index],
            &identities[index],
        )
        .map_err(|error| {
                format!(
                    "{}: inline-rust cpp_abi preflight failed: {error}",
                    carriers[index].path.display()
                )
            })?;
        validate_cpp_abi_host(&carriers[index], &plan)?;
        carriers[index].cpp_abi = Some(plan);
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct CppHostToken {
    text: String,
    offset: usize,
    preprocessor: bool,
    conditional_depth: usize,
}

fn cpp_host_tokens(content: &str, blocks: &[ParsedBlock]) -> Vec<CppHostToken> {
    let bytes = content.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    let mut excluded = 0usize;
    while index < bytes.len() {
        while excluded < blocks.len() && index >= blocks[excluded].replace_end {
            excluded += 1;
        }
        if excluded < blocks.len()
            && index >= blocks[excluded].replace_start
            && index < blocks[excluded].replace_end
        {
            index = blocks[excluded].replace_end;
            continue;
        }
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if bytes[index..].starts_with(b"//") {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            index += 2;
            while index + 1 < bytes.len() && !bytes[index..].starts_with(b"*/") {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            continue;
        }
        if bytes[index] == b'"' || bytes[index] == b'\'' {
            let quote = bytes[index];
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == quote {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
            continue;
        }
        if bytes[index].is_ascii_alphabetic() || bytes[index] == b'_' {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            tokens.push(CppHostToken {
                text: content[start..index].to_string(),
                offset: start,
                preprocessor: cpp_offset_in_preprocessor_directive(content, blocks, start),
                conditional_depth: cpp_preprocessor_depth_at(content, blocks, start),
            });
            continue;
        }
        let start = index;
        index += 1;
        tokens.push(CppHostToken {
            text: content[start..index].to_string(),
            offset: start,
            preprocessor: cpp_offset_in_preprocessor_directive(content, blocks, start),
            conditional_depth: cpp_preprocessor_depth_at(content, blocks, start),
        });
    }
    tokens
}

fn token_sequence_positions(tokens: &[CppHostToken], sequence: &[&str]) -> Vec<usize> {
    if sequence.is_empty() {
        return Vec::new();
    }
    tokens
        .windows(sequence.len())
        .filter(|window| {
            window
                .iter()
                .zip(sequence)
                .all(|(token, expected)| token.text == *expected)
        })
        .map(|window| window[0].offset)
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CppDirectiveIntro {
    Hash,
    Digraph,
}

fn cpp_preprocessor_directive_rest(line: &str) -> Option<(CppDirectiveIntro, &str)> {
    let mut rest = line;
    loop {
        rest = rest.trim_start();
        let Some(comment) = rest.strip_prefix("/*") else {
            break;
        };
        let end = comment.find("*/")?;
        rest = &comment[end + 2..];
    }
    rest = rest.trim_start();
    if let Some(rest) = rest.strip_prefix('#') {
        Some((CppDirectiveIntro::Hash, rest))
    } else {
        rest.strip_prefix("%:")
            .map(|rest| (CppDirectiveIntro::Digraph, rest))
    }
}

fn cpp_preprocessor_directive_parts(
    line: &str,
) -> Option<(CppDirectiveIntro, &str, &str)> {
    let (intro, mut rest) = cpp_preprocessor_directive_rest(line)?;
    rest = rest.trim_start();
    while let Some(comment) = rest.strip_prefix("/*") {
        let end = comment.find("*/")?;
        rest = comment[end + 2..].trim_start();
    }
    let end = rest
        .find(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .unwrap_or(rest.len());
    (end > 0).then_some((intro, &rest[..end], &rest[end..]))
}

#[derive(Clone, Debug)]
struct CppDirectiveRecord {
    start: usize,
    end: usize,
    intro: CppDirectiveIntro,
    keyword: String,
    operand: String,
}

fn cpp_phase3_line(line: &str, in_block_comment: &mut bool) -> String {
    let bytes = line.as_bytes();
    let mut cleaned = bytes.to_vec();
    let mut index = 0usize;
    while index < bytes.len() {
        if *in_block_comment {
            if bytes[index..].starts_with(b"*/") {
                cleaned[index] = b' ';
                cleaned[index + 1] = b' ';
                index += 2;
                *in_block_comment = false;
            } else {
                if !matches!(bytes[index], b'\r' | b'\n') {
                    cleaned[index] = b' ';
                }
                index += 1;
            }
            continue;
        }
        if bytes[index..].starts_with(b"//") {
            while index < bytes.len() {
                if !matches!(bytes[index], b'\r' | b'\n') {
                    cleaned[index] = b' ';
                }
                index += 1;
            }
            break;
        }
        if bytes[index..].starts_with(b"/*") {
            cleaned[index] = b' ';
            cleaned[index + 1] = b' ';
            index += 2;
            *in_block_comment = true;
            continue;
        }
        if matches!(bytes[index], b'\'' | b'"') {
            let quote = bytes[index];
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == quote {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
            continue;
        }
        index += 1;
    }
    String::from_utf8(cleaned).expect("comment replacement preserves UTF-8")
}

fn cpp_preprocessor_directives(
    content: &str,
    blocks: &[ParsedBlock],
) -> Vec<CppDirectiveRecord> {
    let mut records = Vec::new();
    let mut in_block_comment = false;
    for line in collect_line_spans(content) {
        if blocks
            .iter()
            .any(|block| line.start >= block.replace_start && line.start < block.replace_end)
        {
            continue;
        }
        let cleaned = cpp_phase3_line(&content[line.start..line.end], &mut in_block_comment);
        let Some((intro, keyword, operand)) = cpp_preprocessor_directive_parts(&cleaned) else {
            continue;
        };
        records.push(CppDirectiveRecord {
            start: line.start,
            end: line.end,
            intro,
            keyword: keyword.to_string(),
            operand: operand.trim().to_string(),
        });
    }
    records
}

fn cpp_preprocessor_depth_at(content: &str, blocks: &[ParsedBlock], offset: usize) -> usize {
    let mut depth = 0usize;
    for directive in cpp_preprocessor_directives(content, blocks)
        .into_iter()
        .take_while(|directive| directive.start < offset)
    {
        match directive.keyword.as_str() {
            "endif" => depth = depth.saturating_sub(1),
            "if" | "ifdef" | "ifndef" => depth += 1,
            _ => {}
        }
    }
    depth
}

fn cpp_offset_in_preprocessor_directive(
    content: &str,
    blocks: &[ParsedBlock],
    offset: usize,
) -> bool {
    cpp_preprocessor_directives(content, blocks)
        .iter()
        .any(|directive| offset >= directive.start && offset < directive.end)
}

fn validate_cpp_abi_preprocessor_surface(carrier: &LoadedCarrier) -> Result<(), String> {
    let limit = carrier
        .blocks
        .last()
        .map(|block| block.replace_start)
        .unwrap_or(carrier.content.len());
    for line in collect_line_spans(&carrier.content) {
        if line.start >= limit {
            break;
        }
        if carrier
            .blocks
            .iter()
            .any(|block| line.start >= block.replace_start && line.start < block.replace_end)
        {
            continue;
        }
        let source = &carrier.content[line.start..line.end];
        let physical = source.trim_end_matches(['\r', '\n']);
        if source.ends_with('\n') && physical.ends_with('\\') {
            return Err(format!(
                "{}: inline cpp_abi rejects C++ phase-2 line continuations before participating blocks",
                carrier.path.display()
            ));
        }
    }
    if cpp_preprocessor_directives(&carrier.content, &carrier.blocks)
        .iter()
        .any(|directive| {
            directive.start < limit && directive.intro == CppDirectiveIntro::Digraph
        })
    {
            return Err(format!(
                "{}: inline cpp_abi rejects the `%:` preprocessor directive spelling before participating blocks",
                carrier.path.display()
            ));
    }

    let tokens = cpp_host_tokens(&carrier.content, &carrier.blocks);
    validate_cpp_conditional_scope_neutrality(carrier, &tokens, limit)
}

struct CppConditionalFrame {
    parent_dead: bool,
    branch_dead: bool,
    brace_balance: i32,
}

fn cpp_if_operand_is_exact_zero(operand: &str) -> bool {
    matches!(operand, "0" | "(0)")
}

fn validate_cpp_conditional_scope_neutrality(
    carrier: &LoadedCarrier,
    tokens: &[CppHostToken],
    limit: usize,
) -> Result<(), String> {
    let mut stack = Vec::<CppConditionalFrame>::new();
    let directives = cpp_preprocessor_directives(&carrier.content, &carrier.blocks);
    for line in collect_line_spans(&carrier.content) {
        if line.start >= limit {
            break;
        }
        if let Some(directive) = directives.iter().find(|directive| directive.start == line.start) {
            match directive.keyword.as_str() {
            "if" => {
                let parent_dead = stack.last().is_some_and(|frame| frame.branch_dead);
                stack.push(CppConditionalFrame {
                    parent_dead,
                    branch_dead: parent_dead || cpp_if_operand_is_exact_zero(&directive.operand),
                    brace_balance: 0,
                });
            }
            "ifdef" | "ifndef" => {
                let parent_dead = stack.last().is_some_and(|frame| frame.branch_dead);
                stack.push(CppConditionalFrame {
                    parent_dead,
                    branch_dead: parent_dead,
                    brace_balance: 0,
                });
            }
            "elif" | "else" => {
                if let Some(frame) = stack.last_mut() {
                    if !frame.branch_dead && frame.brace_balance != 0 {
                        return Err(format!(
                            "{}: inline cpp_abi requires every conditional host branch before participating blocks to have neutral brace scope",
                            carrier.path.display()
                        ));
                    }
                    frame.brace_balance = 0;
                    frame.branch_dead = frame.parent_dead;
                    if directive.keyword == "elif"
                        && cpp_if_operand_is_exact_zero(&directive.operand)
                    {
                        frame.branch_dead = true;
                    }
                }
            }
            "endif" => {
                if let Some(frame) = stack.pop()
                    && !frame.branch_dead
                    && frame.brace_balance != 0
                {
                    return Err(format!(
                        "{}: inline cpp_abi requires every conditional host branch before participating blocks to have neutral brace scope",
                        carrier.path.display()
                    ));
                }
            }
            _ => {}
            }
            continue;
        }
        if let Some(frame) = stack.last_mut()
            && !frame.branch_dead
        {
            for token in tokens
                .iter()
                .filter(|token| token.offset >= line.start && token.offset < line.end)
            {
                match token.text.as_str() {
                    "{" => frame.brace_balance += 1,
                    "}" => frame.brace_balance -= 1,
                    _ => {}
                }
            }
        }
    }
    for frame in stack {
        if !frame.branch_dead && frame.brace_balance != 0 {
            return Err(format!(
                "{}: inline cpp_abi requires conditional host scope to close before participating blocks",
                carrier.path.display()
            ));
        }
    }
    Ok(())
}

fn cpp_brace_depth_at(tokens: &[CppHostToken], offset: usize) -> usize {
    let mut depth = 0usize;
    for token in tokens.iter().take_while(|token| token.offset < offset) {
        if token.preprocessor || token.conditional_depth != 0 {
            continue;
        }
        match token.text.as_str() {
            "{" => depth += 1,
            "}" => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CppHostScope {
    Namespace { name: String, exported: bool },
    Other,
}

fn export_namespace_scope_at(tokens: &[CppHostToken], offset: usize) -> Option<String> {
    let mut scopes = Vec::new();
    let mut declaration = Vec::<String>::new();
    for token in tokens.iter().take_while(|token| token.offset < offset) {
        if token.preprocessor || token.conditional_depth != 0 {
            continue;
        }
        match token.text.as_str() {
            "{" => {
                let scope = if let Some(namespace) = declaration
                    .iter()
                    .rposition(|part| part == "namespace")
                    .filter(|namespace| *namespace + 1 < declaration.len())
                {
                    let exported = namespace > 0 && declaration[namespace - 1] == "export";
                    CppHostScope::Namespace {
                        name: declaration[namespace + 1..].join(""),
                        exported,
                    }
                } else {
                    CppHostScope::Other
                };
                scopes.push(scope);
                declaration.clear();
            }
            "}" => {
                scopes.pop();
                declaration.clear();
            }
            ";" => declaration.clear(),
            token => declaration.push(token.to_string()),
        }
    }
    let Some(CppHostScope::Namespace { exported: true, .. }) = scopes.last() else {
        return None;
    };
    let mut full = Vec::new();
    for scope in scopes {
        match scope {
            CppHostScope::Namespace { name, .. } => full.push(name),
            CppHostScope::Other => return None,
        }
    }
    Some(full.join("::"))
}

fn cpp_module_name_at(tokens: &[CppHostToken], export_offset: usize) -> Option<String> {
    let start = tokens.iter().position(|token| token.offset == export_offset)?;
    if tokens.get(start)?.text != "export" || tokens.get(start + 1)?.text != "module" {
        return None;
    }
    let mut name = String::new();
    for token in tokens.iter().skip(start + 2) {
        if token.text == ";" {
            return (!name.is_empty()).then_some(name);
        }
        name.push_str(&token.text);
    }
    None
}

fn exact_global_module_import_positions(
    carrier: &LoadedCarrier,
    tokens: &[CppHostToken],
    module: &str,
) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut index = 0usize;
    while index < tokens.len() {
        let at_declaration_boundary = tokens[..index]
            .iter()
            .rev()
            .find(|token| !token.preprocessor)
            .is_none_or(|token| matches!(token.text.as_str(), ";" | "}"));
        if tokens[index].text != "import"
            || tokens[index].preprocessor
            || tokens[index].conditional_depth != 0
            || !at_declaration_boundary
            || cpp_offset_in_preprocessor_directive(
                &carrier.content,
                &carrier.blocks,
                tokens[index].offset,
            )
            || cpp_preprocessor_depth_at(
                &carrier.content,
                &carrier.blocks,
                tokens[index].offset,
            ) != 0
            || cpp_brace_depth_at(tokens, tokens[index].offset) != 0
        {
            index += 1;
            continue;
        }
        let start = tokens[index].offset;
        let mut imported = String::new();
        let mut exact_surface = true;
        index += 1;
        while index < tokens.len() {
            let token = &tokens[index];
            if token.preprocessor
                || token.conditional_depth != 0
                || cpp_offset_in_preprocessor_directive(
                    &carrier.content,
                    &carrier.blocks,
                    token.offset,
                )
                || cpp_preprocessor_depth_at(
                    &carrier.content,
                    &carrier.blocks,
                    token.offset,
                ) != 0
                || cpp_brace_depth_at(tokens, token.offset) != 0
            {
                exact_surface = false;
            }
            if token.text == ";" {
                break;
            }
            imported.push_str(&token.text);
            index += 1;
        }
        if index < tokens.len() && exact_surface && imported == module {
            positions.push(start);
        }
        index += 1;
    }
    positions
}

fn potentially_exported_module_import(
    tokens: &[CppHostToken],
    module: &str,
) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        if token.text != "import" {
            return false;
        }
        let exported = token.preprocessor
            || index
                .checked_sub(1)
                .and_then(|previous| tokens.get(previous))
                .is_some_and(|candidate| candidate.text == "export")
            || tokens[..index]
                .iter()
                .rev()
                .find(|candidate| !candidate.preprocessor)
                .is_some_and(|candidate| candidate.text == "export");
        if !exported {
            return false;
        }
        let mut imported = String::new();
        for candidate in &tokens[index + 1..] {
            if candidate.preprocessor && !token.preprocessor {
                continue;
            }
            if candidate.text == ";" {
                return imported == module;
            }
            imported.push_str(&candidate.text);
        }
        false
    })
}

fn literal_named_module_tokens(tokens: &[CppHostToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let is_identifier = |token: &CppHostToken| {
        let mut characters = token.text.chars();
        characters
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
            && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
    };
    let mut expect_identifier = true;
    let mut saw_partition = false;
    for token in tokens {
        if expect_identifier {
            if !is_identifier(token) {
                return false;
            }
        } else if token.text == "." {
            // A module name and partition may each contain dotted components.
        } else if token.text == ":" && !saw_partition {
            saw_partition = true;
        } else {
            return false;
        }
        expect_identifier = !expect_identifier;
    }
    !expect_identifier
}

fn validate_flat_import_module_zone(
    carrier: &LoadedCarrier,
    tokens: &[CppHostToken],
    module_declaration: usize,
    block_offset: usize,
    scope: &str,
) -> Result<(), String> {
    let module_start = tokens
        .iter()
        .position(|token| token.offset == module_declaration)
        .ok_or_else(|| {
            format!(
                "{}: cannot locate inline cpp_import_namespace module declaration",
                carrier.path.display()
            )
        })?;
    let module_end = (module_start..tokens.len())
        .find(|index| tokens[*index].text == ";")
        .ok_or_else(|| {
            format!(
                "{}: malformed module declaration before cpp_import_namespace block",
                carrier.path.display()
            )
        })?;

    let mut open_braces = Vec::new();
    for (index, token) in tokens.iter().enumerate().take_while(|(_, token)| {
        token.offset < block_offset
    }) {
        if token.preprocessor || token.conditional_depth != 0 {
            continue;
        }
        match token.text.as_str() {
            "{" => open_braces.push(index),
            "}" => {
                open_braces.pop();
            }
            _ => {}
        }
    }
    let namespace_open = open_braces.last().copied().ok_or_else(|| {
        format!(
            "{}: cpp_import_namespace block is not inside its export namespace",
            carrier.path.display()
        )
    })?;
    let namespace_start = (module_end + 1..namespace_open)
        .rev()
        .find(|index| {
            tokens[*index].text == "export"
                && tokens
                    .get(*index + 1)
                    .is_some_and(|token| token.text == "namespace")
                && tokens[*index..namespace_open]
                    .iter()
                    .all(|token| !token.preprocessor && token.conditional_depth == 0)
                && tokens[*index + 2..namespace_open]
                    .iter()
                    .map(|token| token.text.as_str())
                    .collect::<String>()
                    == scope
        })
        .ok_or_else(|| {
            format!(
                "{}: cannot locate exact `export namespace {scope}` opener for cpp_import_namespace block",
                carrier.path.display()
            )
        })?;

    let mut index = module_end + 1;
    while index < namespace_start {
        let token = &tokens[index];
        if token.preprocessor
            || token.conditional_depth != 0
            || token.text != "import"
        {
            return Err(format!(
                "{}: cpp_import_namespace requires the top-level module-import zone to contain only unconditional private literal `import <named.module>;` declarations before `export namespace {scope}`; found `{}`",
                carrier.path.display(),
                token.text
            ));
        }
        let name_start = index + 1;
        index = name_start;
        while index < namespace_start
            && !tokens[index].preprocessor
            && tokens[index].conditional_depth == 0
            && tokens[index].text != ";"
        {
            index += 1;
        }
        if index == namespace_start
            || tokens[index].preprocessor
            || tokens[index].conditional_depth != 0
            || !literal_named_module_tokens(&tokens[name_start..index])
        {
            return Err(format!(
                "{}: cpp_import_namespace requires a complete unconditional private literal named-module import before `export namespace {scope}`",
                carrier.path.display()
            ));
        }
        index += 1;
    }
    Ok(())
}

fn cpp_abi_carrier_identity(carrier: &LoadedCarrier) -> Result<String, String> {
    validate_cpp_abi_preprocessor_surface(carrier)?;
    let tokens = cpp_host_tokens(&carrier.content, &carrier.blocks);
    let mut modules = Vec::new();
    let mut index = 0usize;
    while index + 1 < tokens.len() {
        if tokens[index].text == "export" && tokens[index + 1].text == "module" {
            if cpp_offset_in_preprocessor_directive(
                &carrier.content,
                &carrier.blocks,
                tokens[index].offset,
            )
                || cpp_preprocessor_depth_at(
                    &carrier.content,
                    &carrier.blocks,
                    tokens[index].offset,
                ) != 0
                || cpp_brace_depth_at(&tokens, tokens[index].offset) != 0
            {
                index += 1;
                continue;
            }
            let mut end = index + 2;
            let mut name = String::new();
            while end < tokens.len() && tokens[end].text != ";" {
                name.push_str(&tokens[end].text);
                end += 1;
            }
            if end == tokens.len() || name.is_empty() {
                return Err(format!(
                    "{}: malformed `export module` declaration for inline cpp_abi identity",
                    carrier.path.display()
                ));
            }
            modules.push(name);
            index = end;
        }
        index += 1;
    }
    if modules.len() != 1 {
        return Err(format!(
            "{}: inline cpp_abi requires exactly one C++ module identity; found {}",
            carrier.path.display(),
            modules.len()
        ));
    }
    let provider = carrier
        .blocks
        .iter()
        .find(|block| {
            crate::cpp_abi::source_mentions_reserved_marker(&block.rust_payload_normalized)
        })
        .expect("carrier contract census implies a marker block");
    let scope = export_namespace_scope_at(&tokens, provider.replace_start).ok_or_else(|| {
        format!(
            "{}:{}: inline cpp_abi provider id={} must be directly inside an `export namespace`",
            carrier.path.display(),
            provider.if_line,
            provider.id
        )
    })?;
    let digest = sha256_hex(&format!("{}|{}", modules[0], scope));
    Ok(format!("m_{digest}"))
}

fn exact_global_rusty_header_positions(
    carrier: &LoadedCarrier,
    tokens: &[CppHostToken],
) -> Vec<usize> {
    cpp_preprocessor_directives(&carrier.content, &carrier.blocks)
        .into_iter()
        .filter_map(|directive| {
            if directive.intro != CppDirectiveIntro::Hash
                || directive.keyword != "include"
                || directive.operand != "<rusty/rusty.hpp>"
                || cpp_preprocessor_depth_at(
                    &carrier.content,
                    &carrier.blocks,
                    directive.start,
                ) != 0
                || cpp_brace_depth_at(tokens, directive.start) != 0
            {
                return None;
            }
            Some(directive.start)
        })
        .collect()
}

fn resembles_rusty_umbrella_header_operand(operand: &str) -> bool {
    let compact = operand
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect::<String>();
    compact.starts_with("<rusty/rusty.hpp>") || compact == "<rusty"
}

fn validate_cpp_abi_host(
    carrier: &LoadedCarrier,
    plan: &crate::cpp_abi::CppAbiInlineCarrierPlan,
) -> Result<(), String> {
    if plan.adapted_blocks.is_empty() && plan.flat_import_blocks.is_empty() {
        return Ok(());
    }
    validate_cpp_abi_preprocessor_surface(carrier)?;
    let tokens = cpp_host_tokens(&carrier.content, &carrier.blocks);
    if !plan.flat_import_blocks.is_empty()
        && let Some(token) = tokens.iter().find(|token| {
            token.preprocessor && matches!(token.text.as_str(), "export" | "import")
        })
    {
        return Err(format!(
            "{}: inline cpp_import_namespace rejects preprocessor macro/directive token `{}` because it can assemble a module re-export",
            carrier.path.display(),
            token.text
        ));
    }
    if !plan.flat_import_blocks.is_empty()
        && cpp_preprocessor_directives(&carrier.content, &carrier.blocks)
            .iter()
            .any(|directive| directive.keyword == "define" && directive.operand.contains("##"))
    {
        return Err(format!(
            "{}: inline cpp_import_namespace rejects preprocessor token-pasting because it can assemble a reserved host identifier or module re-export",
            carrier.path.display()
        ));
    }
    if !plan.adapted_blocks.is_empty() {
        if let Some(token) = tokens
            .iter()
            .find(|token| token.text.starts_with("rusty_cpp_abi_"))
        {
            return Err(format!(
                "{}: host C++ identifier `{}` collides with reserved inline cpp_abi generated names",
                carrier.path.display(),
                token.text
            ));
        }
    }

    let module_declarations = token_sequence_positions(&tokens, &["export", "module"])
        .into_iter()
        .filter(|position| {
            !cpp_offset_in_preprocessor_directive(&carrier.content, &carrier.blocks, *position)
                && cpp_preprocessor_depth_at(&carrier.content, &carrier.blocks, *position) == 0
                && cpp_brace_depth_at(&tokens, *position) == 0
        })
        .collect::<Vec<_>>();
    if module_declarations.len() != 1 {
        return Err(format!(
            "{}: inline cpp_abi requires exactly one `export module ...;` declaration; found {}",
            carrier.path.display(),
            module_declarations.len()
        ));
    }
    let module_declaration = module_declarations[0];
    let module_name = cpp_module_name_at(&tokens, module_declaration).ok_or_else(|| {
        format!(
            "{}: malformed `export module` declaration for inline cpp_import_namespace",
            carrier.path.display()
        )
    })?;
    let global_module_fragments = token_sequence_positions(&tokens, &["module", ";"])
        .into_iter()
        .filter(|position| {
            !cpp_offset_in_preprocessor_directive(&carrier.content, &carrier.blocks, *position)
                && cpp_preprocessor_depth_at(&carrier.content, &carrier.blocks, *position) == 0
                && cpp_brace_depth_at(&tokens, *position) == 0
        })
        .collect::<Vec<_>>();
    let std_imports = token_sequence_positions(&tokens, &["import", "std", ";"])
        .into_iter()
        .filter(|position| {
            !cpp_offset_in_preprocessor_directive(&carrier.content, &carrier.blocks, *position)
                && cpp_preprocessor_depth_at(&carrier.content, &carrier.blocks, *position) == 0
                && cpp_brace_depth_at(&tokens, *position) == 0
        })
        .collect::<Vec<_>>();
    let rusty_imports = token_sequence_positions(&tokens, &["import", "rusty", ";"])
        .into_iter()
        .filter(|position| {
            !cpp_offset_in_preprocessor_directive(&carrier.content, &carrier.blocks, *position)
                && cpp_preprocessor_depth_at(&carrier.content, &carrier.blocks, *position) == 0
                && cpp_brace_depth_at(&tokens, *position) == 0
        })
        .collect::<Vec<_>>();
    let rusty_header_candidates =
        cpp_preprocessor_directives(&carrier.content, &carrier.blocks)
            .into_iter()
            .filter(|directive| {
                directive.keyword == "include"
                    && resembles_rusty_umbrella_header_operand(&directive.operand)
            })
            .collect::<Vec<_>>();
    let rusty_headers = exact_global_rusty_header_positions(carrier, &tokens);
    if !plan.adapted_blocks.is_empty()
        && !rusty_header_candidates.is_empty()
        && (rusty_header_candidates.iter().any(|directive| {
            directive.intro != CppDirectiveIntro::Hash
                || directive.operand != "<rusty/rusty.hpp>"
                || cpp_preprocessor_depth_at(
                    &carrier.content,
                    &carrier.blocks,
                    directive.start,
                ) != 0
                || cpp_brace_depth_at(&tokens, directive.start) != 0
        })
            || global_module_fragments.len() != 1
            || tokens
                .iter()
                .any(|token| token.offset < global_module_fragments[0])
            || !rusty_headers.iter().all(|header| {
                global_module_fragments[0] < *header && *header < module_declaration
            }))
    {
        return Err(format!(
            "{}: `<rusty/rusty.hpp>` must be included in one global module fragment (`module;`, then include, then `export module`)",
            carrier.path.display()
        ));
    }

    let mut expected_scope = None::<String>;
    for index in &plan.adapted_blocks {
        let block = &carrier.blocks[*index];
        let offset = block.replace_start;
        if cpp_preprocessor_depth_at(&carrier.content, &carrier.blocks, offset) != 0 {
            return Err(format!(
                "{}:{}: adapted block id={} must be in unconditional host C++",
                carrier.path.display(),
                block.if_line,
                block.id
            ));
        }
        if module_declaration >= offset {
            return Err(format!(
                "{}:{}: adapted block id={} must follow the C++ module declaration",
                carrier.path.display(),
                block.if_line,
                block.id
            ));
        }
        if !std_imports
            .iter()
            .any(|position| module_declaration < *position && *position < offset)
        {
            return Err(format!(
                "{}:{}: adapted block id={} requires a prior exact `import std;`",
                carrier.path.display(),
                block.if_line,
                block.id
            ));
        }
        if !rusty_imports
            .iter()
            .any(|position| module_declaration < *position && *position < offset)
        {
            return Err(format!(
                "{}:{}: adapted block id={} requires a prior exact `import rusty;`; the global-module-fragment `<rusty/rusty.hpp>` header does not export the `rusty::Vec` module alias",
                carrier.path.display(),
                block.if_line,
                block.id
            ));
        }
        let Some(scope) = export_namespace_scope_at(&tokens, offset) else {
            return Err(format!(
                "{}:{}: adapted block id={} must be directly inside an `export namespace`",
                carrier.path.display(),
                block.if_line,
                block.id
            ));
        };
        if let Some(expected) = &expected_scope {
            if expected != &scope {
                return Err(format!(
                    "{}:{}: adapted block id={} is in export namespace `{scope}`, expected `{expected}`",
                    carrier.path.display(),
                    block.if_line,
                    block.id
                ));
            }
        } else {
            expected_scope = Some(scope);
        }
    }

    for index in &plan.flat_import_blocks {
        let block = &carrier.blocks[*index];
        let offset = block.replace_start;
        if cpp_preprocessor_depth_at(&carrier.content, &carrier.blocks, offset) != 0 {
            return Err(format!(
                "{}:{}: cpp_import_namespace block id={} must be in unconditional host C++",
                carrier.path.display(),
                block.if_line,
                block.id
            ));
        }
        if module_declaration >= offset {
            return Err(format!(
                "{}:{}: cpp_import_namespace block id={} must follow the C++ module declaration",
                carrier.path.display(),
                block.if_line,
                block.id
            ));
        }
        let Some(scope) = export_namespace_scope_at(&tokens, offset) else {
            return Err(format!(
                "{}:{}: cpp_import_namespace block id={} must be directly inside an `export namespace`",
                carrier.path.display(),
                block.if_line,
                block.id
            ));
        };
        if let Some(expected) = &expected_scope {
            if expected != &scope {
                return Err(format!(
                    "{}:{}: cpp_import_namespace block id={} is in export namespace `{scope}`, expected `{expected}`",
                    carrier.path.display(),
                    block.if_line,
                    block.id
                ));
            }
        } else {
            expected_scope = Some(scope.clone());
        }
        for (namespace, child, leaves) in plan.flat_import_requirements(*index) {
            if namespace != scope {
                return Err(format!(
                    "{}:{}: cpp_import_namespace `{namespace}` does not match enclosing export namespace `{scope}` for block id={}",
                    carrier.path.display(),
                    block.if_line,
                    block.id
                ));
            }
            let crate_module = module_name.split('.').next().unwrap_or(module_name.as_str());
            let required_module = format!("{crate_module}.{child}");
            if potentially_exported_module_import(&tokens, &required_module) {
                return Err(format!(
                    "{}:{}: cpp_import_namespace block id={} must not export required provider module `{required_module}`",
                    carrier.path.display(),
                    block.if_line,
                    block.id
                ));
            }
            if !exact_global_module_import_positions(carrier, &tokens, &required_module)
                .iter()
                .any(|position| module_declaration < *position && *position < offset)
            {
                return Err(format!(
                    "{}:{}: cpp_import_namespace block id={} requires a prior exact `import {required_module};`",
                    carrier.path.display(),
                    block.if_line,
                    block.id
                ));
            }
            if let Some(token) = tokens.iter().find(|token| {
                leaves.iter().any(|leaf| leaf == &token.text)
            }) {
                return Err(format!(
                    "{}:{}: host C++ identifier `{}` collides with a cpp_import_namespace leaf for block id={}",
                    carrier.path.display(),
                    token.offset,
                    token.text,
                    block.id
                ));
            }
        }
        validate_flat_import_module_zone(
            carrier,
            &tokens,
            module_declaration,
            offset,
            &scope,
        )?;
    }
    Ok(())
}

fn validate_generated_block(path: &Path, block: &ParsedBlock) -> Result<(), String> {
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
    Ok(())
}

fn render_emitted_rust(
    source: &Path,
    blocks: &[ParsedBlock],
    requested_ids: &[String],
) -> Result<(String, usize), String> {
    render_emitted_rust_with_plan(source, blocks, requested_ids, None)
}

fn render_emitted_rust_with_plan(
    source: &Path,
    blocks: &[ParsedBlock],
    requested_ids: &[String],
    plan: Option<&crate::cpp_abi::CppAbiInlineCarrierPlan>,
) -> Result<(String, usize), String> {
    let selected_indices: Vec<usize> = if requested_ids.is_empty() {
        (0..blocks.len()).collect()
    } else if plan.is_none() {
        let mut seen = HashSet::new();
        let mut selected = Vec::with_capacity(requested_ids.len());
        for id in requested_ids {
            if !seen.insert(id.as_str()) {
                return Err(format!("duplicate requested block id={id}"));
            }
            selected.push(
                blocks
                    .iter()
                    .position(|block| block.id == *id)
                    .ok_or_else(|| {
                        format!("{}: missing inline block id={id}", source.display())
                    })?,
            );
        }
        selected
    } else {
        let mut seen = HashSet::new();
        let mut roots = Vec::with_capacity(requested_ids.len());
        for id in requested_ids {
            if !seen.insert(id.as_str()) {
                return Err(format!("duplicate requested block id={id}"));
            }
            let index = blocks
                .iter()
                .position(|block| block.id == *id)
                .ok_or_else(|| format!("{}: missing inline block id={id}", source.display()))?;
            roots.push(index);
        }
        let plan = plan.expect("checked nonempty prepared plan branch");
        fn append_with_dependencies(
            index: usize,
            plan: &crate::cpp_abi::CppAbiInlineCarrierPlan,
            seen: &mut BTreeSet<usize>,
            selected: &mut Vec<usize>,
        ) {
            if seen.contains(&index) {
                return;
            }
            for dependency in &plan.blocks[index].dependencies {
                append_with_dependencies(*dependency, plan, seen, selected);
            }
            if seen.insert(index) {
                selected.push(index);
            }
        }
        let mut selected = Vec::new();
        let mut emitted = BTreeSet::new();
        for root in roots {
            append_with_dependencies(root, plan, &mut emitted, &mut selected);
        }
        selected
    };

    for index in &selected_indices {
        validate_generated_block(source, &blocks[*index])?;
    }

    let mut emitted = selected_indices
        .iter()
        .map(|index| blocks[*index].rust_payload_normalized.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    emitted.push('\n');
    Ok((emitted, selected_indices.len()))
}

fn emit_rust_carrier(
    carrier: &LoadedCarrier,
    output: &Path,
    requested_ids: &[String],
) -> Result<(), String> {
    if carrier.blocks.is_empty() {
        return Err(format!(
            "{}: no inline Rust blocks found to emit",
            carrier.path.display()
        ));
    }
    reject_source_output_alias(&carrier.path, output)?;
    let (emitted, count) = if let Some(plan) = &carrier.cpp_abi {
        render_emitted_rust_with_plan(
            &carrier.path,
            &carrier.blocks,
            requested_ids,
            Some(plan),
        )?
    } else {
        render_emitted_rust(&carrier.path, &carrier.blocks, requested_ids)?
    };

    atomic_write_all(&[(output.to_path_buf(), emitted.into_bytes())])?;
    println!(
        "inline-rust emit-rust: {} -> {} ({} block(s))",
        carrier.path.display(),
        output.display(),
        count
    );
    Ok(())
}

fn check_carriers(carriers: &[LoadedCarrier]) -> Result<(), String> {
    // Rendering is deliberate: --check exercises the same prepared lowering
    // and code-generation path as --rewrite before validating stored hashes.
    for carrier in carriers {
        if carrier.blocks.is_empty() {
            println!(
                "inline-rust skip: {} (no block markers)",
                carrier.path.display()
            );
            continue;
        }
        let rust_context = inline_rust_context(&carrier.path, &carrier.blocks)?;
        let _ = rewrite_content_with_plan_and_context(
            &carrier.path,
            &carrier.content,
            &carrier.blocks,
            carrier.cpp_abi.as_ref(),
            &rust_context,
        )?;
        for block in &carrier.blocks {
            validate_generated_block(&carrier.path, block)?;
        }
        println!(
            "inline-rust check: {} ({} block(s))",
            carrier.path.display(),
            carrier.blocks.len()
        );
    }
    Ok(())
}

fn rewrite_carriers(carriers: &[LoadedCarrier]) -> Result<(), String> {
    // Complete every parse, ABI preflight, and render before staging any file.
    let mut writes = Vec::new();
    for carrier in carriers {
        if carrier.blocks.is_empty() {
            continue;
        }
        let rust_context = inline_rust_context(&carrier.path, &carrier.blocks)?;
        let rewritten = rewrite_content_with_plan_and_context(
            &carrier.path,
            &carrier.content,
            &carrier.blocks,
            carrier.cpp_abi.as_ref(),
            &rust_context,
        )?;
        if rewritten != carrier.content {
            writes.push((carrier.path.clone(), rewritten.into_bytes()));
        }
    }
    atomic_write_all(&writes)?;
    for carrier in carriers {
        if carrier.blocks.is_empty() {
            println!(
                "inline-rust skip: {} (no block markers)",
                carrier.path.display()
            );
        } else {
            println!(
                "inline-rust rewrite: {} ({} block(s))",
                carrier.path.display(),
                carrier.blocks.len()
            );
        }
    }
    Ok(())
}

fn atomic_write_all(writes: &[(PathBuf, Vec<u8>)]) -> Result<(), String> {
    let mut staged = Vec::<(PathBuf, PathBuf)>::new();
    for (target, bytes) in writes {
        let parent = target.parent().unwrap_or_else(|| Path::new("."));
        let file_name = target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("inline-rust-output");
        let mut staged_one = None;
        for sequence in 0..1024u32 {
            let temporary = parent.join(format!(
                ".{file_name}.rusty-cpp-tmp-{}-{sequence}",
                std::process::id()
            ));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
            {
                Ok(mut file) => {
                    let result = (|| -> std::io::Result<()> {
                        file.write_all(bytes)?;
                        file.sync_all()?;
                        if let Ok(metadata) = fs::metadata(target) {
                            fs::set_permissions(&temporary, metadata.permissions())?;
                        }
                        Ok(())
                    })();
                    if let Err(error) = result {
                        let _ = fs::remove_file(&temporary);
                        for (staged_path, _) in &staged {
                            let _ = fs::remove_file(staged_path);
                        }
                        return Err(format!(
                            "{}: failed to stage atomic inline-rust write: {}",
                            target.display(),
                            error
                        ));
                    }
                    staged_one = Some(temporary);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    for (staged_path, _) in &staged {
                        let _ = fs::remove_file(staged_path);
                    }
                    return Err(format!(
                        "{}: failed to create atomic inline-rust temporary: {}",
                        target.display(),
                        error
                    ));
                }
            }
        }
        let temporary = staged_one.ok_or_else(|| {
            format!(
                "{}: exhausted atomic inline-rust temporary names",
                target.display()
            )
        })?;
        staged.push((temporary, target.clone()));
    }

    for (temporary, target) in &staged {
        if let Err(error) = fs::rename(temporary, target) {
            for (staged_path, _) in &staged {
                let _ = fs::remove_file(staged_path);
            }
            return Err(format!(
                "{}: failed to install atomic inline-rust output: {}",
                target.display(),
                error
            ));
        }
    }
    Ok(())
}

fn reject_source_output_alias(source: &Path, output: &Path) -> Result<(), String> {
    let canonical_source = fs::canonicalize(source)
        .map_err(|e| format!("{}: failed to resolve source path: {}", source.display(), e))?;

    let canonical_output =
        if output.exists() {
            Some(fs::canonicalize(output).map_err(|e| {
                format!("{}: failed to resolve output path: {}", output.display(), e)
            })?)
        } else {
            let parent = output
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            let file_name = output
                .file_name()
                .ok_or_else(|| format!("{}: output path has no file name", output.display()))?;
            fs::canonicalize(parent)
                .ok()
                .map(|canonical_parent| canonical_parent.join(file_name))
        };

    if canonical_output.as_ref() == Some(&canonical_source) {
        return Err(format!(
            "refusing to emit Rust over source file {}",
            source.display()
        ));
    }

    #[cfg(unix)]
    if output.exists() {
        use std::os::unix::fs::MetadataExt;
        let source_metadata = fs::metadata(source)
            .map_err(|e| format!("{}: failed to stat source: {}", source.display(), e))?;
        let output_metadata = fs::metadata(output)
            .map_err(|e| format!("{}: failed to stat output: {}", output.display(), e))?;
        if source_metadata.dev() == output_metadata.dev()
            && source_metadata.ino() == output_metadata.ino()
        {
            return Err(format!(
                "refusing to emit Rust over source file {}",
                source.display()
            ));
        }
    }

    Ok(())
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
    // Every id already present in a GEN marker anywhere in the file --
    // including ones we have not reached yet. Auto-id minting must skip
    // these: ids are positional (`file.N`), so inserting a NEW block in the
    // MIDDLE of a file otherwise mints an id a downstream marker already
    // owns and parsing dies with "duplicate inline block id". Existing
    // blocks keep their marker ids; only new blocks mint, and they mint the
    // first index not taken anywhere in the file.
    let all_marker_ids: HashSet<String> = lines
        .iter()
        .filter_map(|span| parse_gen_begin_marker(line_trimmed(content, span)))
        .map(|m| m.id)
        .collect();
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
            None => {
                let mut n = blocks.len() + 1;
                loop {
                    let candidate = make_auto_id(path, n);
                    if !all_marker_ids.contains(&candidate) && !seen_ids.contains(&candidate) {
                        break candidate;
                    }
                    n += 1;
                }
            }
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
/// Collect C++ `using X = Y;` alias declarations from the surrounding
/// translation unit.
///
/// The DSL blocks are spliced into a real C++ file, and that file's
/// aliases are part of the context the block is written against. mako
/// spells its captures `WeakClientConnection`, a `using` alias for
/// `rusty::sync::Weak<ClientConnection>`; without the alias the
/// pointer-like predicate matches the last path segment by NAME, sees
/// "WeakClientConnection", and cannot tell it is a Weak. That made the
/// closure-mutability analysis inert on exactly the code it exists for.
///
/// Deliberately a line scanner, not a C++ parser: it recognises the one
/// shape that matters (`using NAME = TARGET;` on one line) and ignores
/// everything else. A missed alias degrades to today's behaviour.
fn collect_cpp_type_aliases(content: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for line in content.lines() {
        let t = line.trim();
        let Some(rest) = t.strip_prefix("using ") else {
            continue;
        };
        let Some((name, target)) = rest.split_once('=') else {
            continue;
        };
        let name = name.trim();
        let target = target.trim().trim_end_matches(';').trim();
        // `using X = Y;` only — skip `using namespace N;` and
        // using-DECLARATIONS (`using rusty::foo;`), which have no `=`.
        if name.is_empty() || target.is_empty() || name.contains(|c: char| !(c.is_alphanumeric() || c == '_')) {
            continue;
        }
        out.insert(name.to_string(), target.to_string());
    }
    out
}

fn collect_file_cpp_inherit(
    blocks: &[ParsedBlock],
    rust_context: &InlineRustContext,
) -> Vec<(syn::ItemStruct, String)> {
    // (struct def, trait) for every `#[cpp_inherit] impl Trait for Type`
    // in any block — a sibling block constructing such a type must use
    // its fieldwise ctor (the emitted C++ struct has a base class, so
    // designated init is illegal), and needs the field order to build
    // the positional argument list. Only this harvest tells it either.
    let mut structs = std::collections::HashMap::new();
    let mut impls = Vec::new();
    for block in blocks {
        let Ok(file) = syn::parse_file(&block.rust_payload_normalized) else {
            continue;
        };
        for item in file.items {
            match item {
                syn::Item::Struct(item_struct) => {
                    structs.insert(item_struct.ident.to_string(), item_struct);
                }
                syn::Item::Impl(item_impl) => {
                    let has_attr = crate::transpile::has_authenticated_cpp_inherit_attr(
                        &item_impl.attrs,
                        &[],
                        &rust_context.import_bindings,
                        &rust_context.authenticated_cpp_inherit_roots,
                    );
                    if has_attr {
                        impls.push(item_impl);
                    }
                }
                _ => {}
            }
        }
    }
    let mut pairs = Vec::new();
    for item_impl in impls {
        let Some((_, trait_path, _)) = &item_impl.trait_ else {
            continue;
        };
        let Some(trait_seg) = trait_path.segments.last() else {
            continue;
        };
        let syn::Type::Path(tp) = &*item_impl.self_ty else {
            continue;
        };
        let Some(type_seg) = tp.path.segments.last() else {
            continue;
        };
        if let Some(item_struct) = structs.get(&type_seg.ident.to_string()) {
            pairs.push((item_struct.clone(), trait_seg.ident.to_string()));
        }
    }
    pairs
}

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
    cpp_aliases: &std::collections::HashMap<String, String>,
    file_cpp_inherit: &[(syn::ItemStruct, String)],
    rust_context: &InlineRustContext,
) -> Result<String, String> {
    let generated_cpp = transpile_payload_to_cpp(
        block,
        file_enums,
        cpp_aliases,
        file_cpp_inherit,
        rust_context,
        None,
    )?;
    Ok(render_generated_region_with_cpp(block, &generated_cpp))
}

fn render_generated_region_prepared(
    block: &ParsedBlock,
    file_enums: &[syn::ItemEnum],
    cpp_aliases: &std::collections::HashMap<String, String>,
    file_cpp_inherit: &[(syn::ItemStruct, String)],
    rust_context: &InlineRustContext,
    prepared: &crate::cpp_abi::CppAbiInlineBlockPlan,
) -> Result<String, String> {
    let generated_cpp = transpile_payload_to_cpp(
        block,
        file_enums,
        cpp_aliases,
        file_cpp_inherit,
        rust_context,
        Some(prepared),
    )?;
    Ok(render_generated_region_with_cpp(block, &generated_cpp))
}

fn transpile_payload_to_cpp(
    block: &ParsedBlock,
    file_enums: &[syn::ItemEnum],
    cpp_aliases: &std::collections::HashMap<String, String>,
    file_cpp_inherit: &[(syn::ItemStruct, String)],
    rust_context: &InlineRustContext,
    prepared: Option<&crate::cpp_abi::CppAbiInlineBlockPlan>,
) -> Result<String, String> {
    let options = transpile::TranspileOptions {
        // Sibling blocks in the same file are this block's module scope.
        cross_file_enums: file_enums.to_vec(),
        cross_file_cpp_inherit: file_cpp_inherit.to_vec(),
        cross_file_rust_item_import_bindings: rust_context.import_bindings.clone(),
        authenticated_cpp_inherit_roots:
            rust_context.authenticated_cpp_inherit_roots.clone(),
        authenticated_sysroot_roots: rust_context.authenticated_sysroot_roots.clone(),
        // Surrounding-TU `using` aliases, so pointer-like detection can see
        // through `WeakClientConnection` to `rusty::sync::Weak<..>`.
        cpp_type_aliases: cpp_aliases.clone(),
        // Inline-rust blocks are spliced into a TU that `import rusty;`, and may
        // sit inside a consumer namespace — suppress the redundant runtime
        // preamble that would otherwise shadow `::rusty`.
        inline_rust_block: true,
        ..transpile::TranspileOptions::default()
    };
    let generated = if let Some(prepared) = prepared {
        transpile::transpile_prepared_inline_cpp_abi(
            prepared.lowered.clone(),
            prepared.emission.clone(),
            &UserTypeMap::default(),
            &HashSet::new(),
            &options,
        )?
    } else {
        transpile::transpile_full_with_options(
            &block.rust_payload_normalized,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )?
    };
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
    cpp_aliases: &std::collections::HashMap<String, String>,
    file_cpp_inherit: &[(syn::ItemStruct, String)],
    rust_context: &InlineRustContext,
) -> Result<String, String> {
    let mut out = String::new();
    out.push_str(&render_rust_block(block));
    out.push_str(&render_generated_region(
        block,
        file_enums,
        cpp_aliases,
        file_cpp_inherit,
        rust_context,
    )?);
    Ok(out)
}

fn render_block_rewrite_prepared(
    block: &ParsedBlock,
    file_enums: &[syn::ItemEnum],
    cpp_aliases: &std::collections::HashMap<String, String>,
    file_cpp_inherit: &[(syn::ItemStruct, String)],
    rust_context: &InlineRustContext,
    prepared: &crate::cpp_abi::CppAbiInlineBlockPlan,
) -> Result<String, String> {
    let mut out = String::new();
    out.push_str(&render_rust_block(block));
    out.push_str(&render_generated_region_prepared(
        block,
        file_enums,
        cpp_aliases,
        file_cpp_inherit,
        rust_context,
        prepared,
    )?);
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
    rewrite_content_with_plan(path, content, blocks, None)
}

fn rewrite_content_with_plan(
    path: &Path,
    content: &str,
    blocks: &[ParsedBlock],
    cpp_abi: Option<&crate::cpp_abi::CppAbiInlineCarrierPlan>,
) -> Result<String, String> {
    rewrite_content_with_plan_and_context(
        path,
        content,
        blocks,
        cpp_abi,
        &InlineRustContext::default(),
    )
}

fn rewrite_content_with_plan_and_context(
    path: &Path,
    content: &str,
    blocks: &[ParsedBlock],
    cpp_abi: Option<&crate::cpp_abi::CppAbiInlineCarrierPlan>,
    rust_context: &InlineRustContext,
) -> Result<String, String> {
    if blocks.is_empty() {
        return Ok(content.to_string());
    }
    if let Some(plan) = cpp_abi
        && plan.blocks.len() != blocks.len()
    {
        return Err(format!(
            "{}: internal inline cpp_abi block-plan census mismatch ({} blocks, {} plans)",
            path.display(),
            blocks.len(),
            plan.blocks.len()
        ));
    }

    let file_enums = collect_file_enums(blocks);
    let cpp_aliases = collect_cpp_type_aliases(content);
    let file_cpp_inherit = collect_file_cpp_inherit(blocks, rust_context);

    let mut out = String::with_capacity(content.len() + blocks.len() * 128);
    let mut cursor = 0usize;
    let mut dispatch_methods: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for (index, block) in blocks.iter().enumerate() {
        out.push_str(&content[cursor..block.replace_start]);
        let mut rewritten = match cpp_abi {
            Some(plan) => render_block_rewrite_prepared(
                block,
                &file_enums,
                &cpp_aliases,
                &file_cpp_inherit,
                rust_context,
                &plan.blocks[index],
            ),
            None => render_block_rewrite(
                block,
                &file_enums,
                &cpp_aliases,
                &file_cpp_inherit,
                rust_context,
            ),
        }
        .map_err(|e| {
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

    fn cpp_abi_carrier(blocks: &[&str]) -> String {
        let mut out = String::from(
            "module;\n#include <rusty/rusty.hpp>\nexport module demo;\nimport std;\nimport rusty;\nexport namespace rrr {\n",
        );
        for block in blocks {
            out.push_str("#if RUSTYCPP_RUST\n");
            out.push_str(block);
            if !block.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("#endif\n");
        }
        out.push_str("} // namespace rrr\n");
        out
    }

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

    #[test]
    fn cpp_abi_inline_rewrites_earlier_provider_and_emits_support_once() {
        let content = cpp_abi_carrier(&[
            r#"
#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
pub fn zero_pad(v: Vec<u8>) -> Vec<u8> { v }
"#,
            "pub fn format(v: Vec<u8>) -> Vec<u8> { zero_pad(v) }\n",
        ]);
        let path = PathBuf::from("demo.cpp");
        let blocks = parse_blocks(&path, &content).unwrap();
        let mut carriers = vec![LoadedCarrier {
            path: path.clone(),
            content: content.clone(),
            blocks,
            cpp_abi: None,
        }];
        prepare_cpp_abi_carriers(&mut carriers).unwrap();
        let output = rewrite_content_with_plan(
            &path,
            &content,
            &carriers[0].blocks,
            carriers[0].cpp_abi.as_ref(),
        )
        .unwrap();
        assert_eq!(
            output
                .matches("\nnamespace rusty_cpp_abi_detail_m_")
                .count(),
            1,
            "conversion support must have one owner:\n{output}"
        );
        assert!(
            output.contains("inline rusty::Vec<uint8_t> bytes_from_std_string"),
            "{output}"
        );
        assert!(
            output.contains("std::string zero_pad(std::string v)"),
            "{output}"
        );
        assert!(
            !output.contains("inline std::string zero_pad(std::string v)"),
            "public ABI facade must remain a strong non-inline definition:\n{output}"
        );
        assert!(
            output.contains("_zero_pad(std::move(v))"),
            "later call must bind the semantic helper:\n{output}"
        );
        assert!(!output.contains("static inline rusty_cpp_abi_sem_"));
    }

    #[test]
    fn cpp_abi_inline_keeps_static_method_facade_non_inline() {
        let content = cpp_abi_carrier(&[r#"
pub struct Codec;
impl Codec {
    #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
    pub fn encode(value: u8) -> Vec<u8> {
        let mut output = Vec::new();
        output.push(value);
        output
    }
}
"#]);
        let path = PathBuf::from("method.cpp");
        let blocks = parse_blocks(&path, &content).unwrap();
        let mut carriers = vec![LoadedCarrier {
            path: path.clone(),
            content: content.clone(),
            blocks,
            cpp_abi: None,
        }];
        prepare_cpp_abi_carriers(&mut carriers).unwrap();
        let output = rewrite_content_with_plan(
            &path,
            &content,
            &carriers[0].blocks,
            carriers[0].cpp_abi.as_ref(),
        )
        .unwrap();
        assert!(output.contains("static std::string encode(uint8_t value);"), "{output}");
        assert!(
            output.contains("std::string Codec::encode(uint8_t value) {"),
            "{output}"
        );
        assert!(
            !output.contains("inline std::string Codec::encode(uint8_t value)"),
            "public static-method facade must remain a strong definition:\n{output}"
        );
        assert!(output.contains("inline rusty::Vec<uint8_t> rusty_cpp_abi_sem_"));
    }

    #[test]
    fn cpp_abi_inline_multiple_provider_blocks_emit_no_empty_support_namespace() {
        let content = cpp_abi_carrier(&[
            r#"
#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
pub fn first_bytes(v: Vec<u8>) -> Vec<u8> { v }
"#,
            r#"
#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
pub fn second_bytes(v: Vec<u8>) -> Vec<u8> { v }
"#,
        ]);
        let path = PathBuf::from("two-providers.cpp");
        let blocks = parse_blocks(&path, &content).unwrap();
        let mut carriers = vec![LoadedCarrier {
            path: path.clone(),
            content: content.clone(),
            blocks,
            cpp_abi: None,
        }];
        prepare_cpp_abi_carriers(&mut carriers).unwrap();
        let output = rewrite_content_with_plan(
            &path,
            &content,
            &carriers[0].blocks,
            carriers[0].cpp_abi.as_ref(),
        )
        .unwrap();
        assert_eq!(
            output
                .matches("\nnamespace rusty_cpp_abi_detail_m_")
                .count(),
            1,
            "only the first provider may own conversion support:\n{output}"
        );
        assert!(output.contains("std::string first_bytes(std::string v)"));
        assert!(output.contains("std::string second_bytes(std::string v)"));
    }

    #[test]
    fn cpp_import_namespace_inline_is_flat_private_and_adapter_free() {
        let content = r#"export module rrr.consumer;
import rrr.rand;
export namespace rrr {
#if RUSTYCPP_RUST
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::rand::{randgen_rand_max, randgen_rand_raw};
pub fn draw() -> f64 {
    randgen_rand_raw() as f64 / randgen_rand_max()
}
#endif
} // namespace rrr
"#
        .to_string();
        let path = PathBuf::from("flat-only.cppm");
        let blocks = parse_blocks(&path, &content).unwrap();
        let mut carriers = vec![LoadedCarrier {
            path: path.clone(),
            content: content.clone(),
            blocks,
            cpp_abi: None,
        }];
        prepare_cpp_abi_carriers(&mut carriers).unwrap();
        let plan = carriers[0].cpp_abi.as_ref().unwrap();
        assert!(plan.adapted_blocks.is_empty());
        assert_eq!(plan.flat_import_blocks, BTreeSet::from([0]));

        let output = rewrite_content_with_plan(
            &path,
            &content,
            &carriers[0].blocks,
            Some(plan),
        )
        .unwrap();
        assert_eq!(output.matches("import rrr.rand;").count(), 1, "{output}");
        assert!(!output.contains("using ::rrr::"), "{output}");
        assert!(!output.contains("namespace rand ="), "{output}");
        assert!(!output.contains("rusty_cpp_abi_detail"), "{output}");
        assert!(!output.contains("rusty_cpp_abi_sem"), "{output}");
        assert!(!output.contains("import rusty;"), "{output}");
        assert!(output.contains("randgen_rand_raw()"), "{output}");

        let comment_and_literal = content.replace(
            "export namespace rrr {",
            "export namespace rrr {\n// randgen_rand_raw is mentioned only in a comment\nconstexpr char flat_import_note[] = \"randgen_rand_max\";",
        );
        let blocks = parse_blocks(Path::new("comment-and-literal"), &comment_and_literal).unwrap();
        let mut control_carriers = vec![LoadedCarrier {
            path: PathBuf::from("comment-and-literal"),
            content: comment_and_literal,
            blocks,
            cpp_abi: None,
        }];
        prepare_cpp_abi_carriers(&mut control_carriers).unwrap();

        for (label, broken, expected) in [
            (
                "missing import",
                content.replace("import rrr.rand;\n", ""),
                "requires a prior exact `import rrr.rand;`",
            ),
            (
                "different import",
                content.replace("import rrr.rand;", "import rrr.other;"),
                "requires a prior exact `import rrr.rand;`",
            ),
            (
                "exported import",
                content.replace("import rrr.rand;", "export import rrr.rand;"),
                "must not export required provider module `rrr.rand`",
            ),
            (
                "private plus exported import",
                content.replace(
                    "import rrr.rand;",
                    "import rrr.rand;\nexport import rrr.rand;",
                ),
                "must not export required provider module `rrr.rand`",
            ),
            (
                "private plus late exported import",
                format!("{content}\nexport import rrr.rand;\n"),
                "must not export required provider module `rrr.rand`",
            ),
            (
                "private plus conditional exported import",
                content.replace(
                    "import rrr.rand;",
                    "import rrr.rand;\n#if ENABLE_RAND\nexport import rrr.rand;\n#endif",
                ),
                "must not export required provider module `rrr.rand`",
            ),
            (
                "private plus conditionally split exported import",
                content.replace(
                    "import rrr.rand;",
                    "import rrr.rand;\n#if ENABLE_RAND\nexport\n#endif\nimport rrr.rand;",
                ),
                "must not export required provider module `rrr.rand`",
            ),
            (
                "private plus macro-assembled exported import",
                content.replace(
                    "import rrr.rand;",
                    "import rrr.rand;\n#define REEXPORT export import rrr.rand;\nREEXPORT",
                ),
                "can assemble a module re-export",
            ),
            (
                "private plus split macro-assembled exported import",
                content.replace(
                    "import rrr.rand;",
                    "import rrr.rand;\n#define E export\n#define I import\nE I rrr.rand;",
                ),
                "can assemble a module re-export",
            ),
            (
                "private plus command-line-style reexport invocation",
                content.replace(
                    "import rrr.rand;",
                    "import rrr.rand;\nREEXPORT",
                ),
                "top-level module-import zone",
            ),
            (
                "private plus token-pasted reexport invocation",
                content
                    .replace(
                        "export module rrr.consumer;",
                        "#define CAT_(a, b) a ## b\n#define CAT(a, b) CAT_(a, b)\nexport module rrr.consumer;",
                    )
                    .replace(
                        "import rrr.rand;",
                        "import rrr.rand;\nCAT(ex, port) CAT(im, port) rrr.rand;",
                    ),
                "rejects preprocessor token-pasting",
            ),
            (
                "private plus provider-macro reexport invocation",
                content
                    .replace(
                        "export module rrr.consumer;",
                        "#define PROVIDER rrr.rand\nexport module rrr.consumer;",
                    )
                    .replace(
                        "import rrr.rand;",
                        "import rrr.rand;\nexport import PROVIDER;",
                    ),
                "top-level module-import zone",
            ),
            (
                "conditionally split exported import",
                content.replace(
                    "import rrr.rand;",
                    "#if ENABLE_RAND\nexport\n#endif\nimport rrr.rand;",
                ),
                "must not export required provider module `rrr.rand`",
            ),
            (
                "conditionally split import semicolon",
                content.replace(
                    "import rrr.rand;",
                    "import rrr.rand\n#if ENABLE_RAND\n;\n#endif",
                ),
                "requires a prior exact `import rrr.rand;`",
            ),
            (
                "late import",
                format!(
                    "{}import rrr.rand;\n",
                    content.replace("import rrr.rand;\n", "")
                ),
                "requires a prior exact `import rrr.rand;`",
            ),
            (
                "conditional import",
                content.replace(
                    "import rrr.rand;",
                    "#if ENABLE_RAND\nimport rrr.rand;\n#endif",
                ),
                "requires a prior exact `import rrr.rand;`",
            ),
            (
                "namespace mismatch",
                content.replace("cpp_import_namespace(rrr)", "cpp_import_namespace(other)"),
                "does not match enclosing export namespace",
            ),
            (
                "full namespace mismatch",
                content
                    .replace("export namespace rrr {", "export namespace outer::rrr {")
                    .replace(
                        "cpp_import_namespace(rrr)",
                        "cpp_import_namespace(outer)",
                ),
                "does not match enclosing export namespace",
            ),
            (
                "marker-only host line splice",
                content.replace("import rrr.rand;\n", "import rrr.rand;\\\n"),
                "line continuations",
            ),
            (
                "marker-only host digraph",
                content.replace(
                    "import rrr.rand;",
                    "%:if 0\nimport rrr.rand;\n#endif",
                ),
                "`%:`",
            ),
            (
                "marker-only conditional brace",
                content
                    .replace(
                        "export namespace rrr {",
                        "#if FEATURE\nnamespace outer {\n#endif\nexport namespace rrr {",
                    )
                    .replace(
                        "} // namespace rrr",
                        "} // namespace rrr\n#if FEATURE\n}\n#endif",
                    ),
                "neutral brace scope",
            ),
            (
                "host macro captures imported leaf",
                content.replace(
                    "export namespace rrr {",
                    "#define randgen_rand_raw() 7\nexport namespace rrr {",
                ),
                "host C++ identifier `randgen_rand_raw` collides",
            ),
            (
                "host token-pasted declaration captures imported leaf",
                content
                    .replace(
                        "export module rrr.consumer;",
                        "#define CAT_(a, b) a ## b\n#define CAT(a, b) CAT_(a, b)\nexport module rrr.consumer;",
                    )
                    .replace(
                        "export namespace rrr {",
                        "export namespace rrr {\ninline unsigned long long CAT(randgen_rand_, raw)() { return 7; }",
                    ),
                "rejects preprocessor token-pasting",
            ),
            (
                "host declaration shadows imported leaf",
                content.replace(
                    "export namespace rrr {",
                    "export namespace rrr {\ninline unsigned long long randgen_rand_raw() { return 7; }",
                ),
                "host C++ identifier `randgen_rand_raw` collides",
            ),
            (
                "host alias shadows imported leaf",
                content.replace(
                    "export namespace rrr {",
                    "export namespace rrr {\nusing randgen_rand_max = unsigned long long;",
                ),
                "host C++ identifier `randgen_rand_max` collides",
            ),
        ] {
            let blocks = parse_blocks(Path::new(label), &broken).unwrap();
            let original = broken.clone();
            let mut broken_carriers = vec![LoadedCarrier {
                path: PathBuf::from(label),
                content: broken,
                blocks,
                cpp_abi: None,
            }];
            let error = prepare_cpp_abi_carriers(&mut broken_carriers).expect_err(label);
            assert!(error.contains(expected), "{label}: {error}");
            assert_eq!(broken_carriers[0].content, original, "{label}");
        }
    }

    #[test]
    fn cpp_abi_inline_rejects_backward_calls_and_wrong_host_scope() {
        let backward = cpp_abi_carrier(&[
            "pub fn format(v: Vec<u8>) -> Vec<u8> { zero_pad(v) }\n",
            r#"
#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
pub fn zero_pad(v: Vec<u8>) -> Vec<u8> { v }
"#,
        ]);
        let path = PathBuf::from("backward.cpp");
        let blocks = parse_blocks(&path, &backward).unwrap();
        let mut carriers = vec![LoadedCarrier {
            path,
            content: backward,
            blocks,
            cpp_abi: None,
        }];
        let error = prepare_cpp_abi_carriers(&mut carriers).expect_err("backward call");
        assert!(error.contains("zero_pad"), "{error}");

        let mismatched = cpp_abi_carrier(&[
            r#"
#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
pub fn zero_pad(v: Vec<u8>) -> Vec<u8> { v }
"#,
            "pub fn format(v: Vec<u8>) -> Vec<u8> { zero_pad(v) }\n",
        ])
        .replacen(
            "#if RUSTYCPP_RUST\npub fn format",
            "} // namespace rrr\nexport namespace other {\n#if RUSTYCPP_RUST\npub fn format",
            1,
        );
        let path = PathBuf::from("scope.cpp");
        let blocks = parse_blocks(&path, &mismatched).unwrap();
        let mut carriers = vec![LoadedCarrier {
            path,
            content: mismatched,
            blocks,
            cpp_abi: None,
        }];
        let error = prepare_cpp_abi_carriers(&mut carriers).expect_err("scope mismatch");
        assert!(error.contains("expected `rrr`"), "{error}");
    }

    #[test]
    fn cpp_abi_emit_rust_selection_includes_earlier_dependencies() {
        let content = cpp_abi_carrier(&[
            r#"
#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
pub fn zero_pad(v: Vec<u8>) -> Vec<u8> { v }
"#,
            "pub fn format(v: Vec<u8>) -> Vec<u8> { zero_pad(v) }\n",
        ]);
        let path = PathBuf::from("emit.cpp");
        let blocks = parse_blocks(&path, &content).unwrap();
        let files = blocks
            .iter()
            .map(|block| syn::parse_file(&block.rust_payload_normalized).unwrap())
            .collect::<Vec<_>>();
        let plan = crate::cpp_abi::prepare_inline_carrier(
            &files,
            &crate::cpp_abi::ExternalContractIndex::default(),
            "test",
        )
        .unwrap();
        // Generated-marker validation is orthogonal here; pin dependency
        // closure directly using valid synthetic metadata.
        let mut blocks = blocks;
        for block in &mut blocks {
            block.generated_region = Some(GenRegion {
                end_line: 0,
                id: block.id.clone(),
                version: "1".to_string(),
                rust_sha256: block.rust_hash.clone(),
            });
        }
        let consumer_id = blocks[1].id.clone();
        let (emitted, count) = render_emitted_rust_with_plan(
            &path,
            &blocks,
            &[consumer_id],
            Some(&plan),
        )
        .unwrap();
        assert_eq!(count, 2);
        assert!(emitted.find("fn zero_pad").unwrap() < emitted.find("fn format").unwrap());
    }

    #[test]
    fn cpp_abi_cross_carrier_allows_bound_names_but_rejects_helper_collisions() {
        let first = cpp_abi_carrier(&[r#"
#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
pub fn foo(v: Vec<u8>) {}
"#]);
        let harmless = cpp_abi_carrier(&[r#"
pub struct Other;
impl Other { pub fn foo() -> i32 { 7 } }
pub fn local() -> i32 { let foo = 7; foo + Other::foo() }
"#]);
        let mut carriers = [
            LoadedCarrier {
                path: PathBuf::from("first.cpp"),
                blocks: parse_blocks(Path::new("first.cpp"), &first).unwrap(),
                content: first,
                cpp_abi: None,
            },
            LoadedCarrier {
                path: PathBuf::from("harmless.cpp"),
                blocks: parse_blocks(Path::new("harmless.cpp"), &harmless).unwrap(),
                content: harmless,
                cpp_abi: None,
            },
        ];
        prepare_cpp_abi_carriers(&mut carriers).expect("lexically bound local is unrelated");

        let free = cpp_abi_carrier(&[r#"
#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
pub fn A_b(v: Vec<u8>) {}
"#]);
        let method_same_module = cpp_abi_carrier(&[r#"
pub struct A;
impl A {
    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
    pub fn b(v: Vec<u8>) {}
}
"#]);
        let method_other_module = method_same_module.replacen(
            "export module demo;",
            "export module demo_method;",
            1,
        );
        let mut distinct = [
            LoadedCarrier {
                path: PathBuf::from("free.cpp"),
                blocks: parse_blocks(Path::new("free.cpp"), &free).unwrap(),
                content: free.clone(),
                cpp_abi: None,
            },
            LoadedCarrier {
                path: PathBuf::from("method-other.cpp"),
                blocks: parse_blocks(Path::new("method-other.cpp"), &method_other_module).unwrap(),
                content: method_other_module,
                cpp_abi: None,
            },
        ];
        prepare_cpp_abi_carriers(&mut distinct)
            .expect("module-unique generated helper spellings must not collide");

        let mut carriers = [
            LoadedCarrier {
                path: PathBuf::from("free.cpp"),
                blocks: parse_blocks(Path::new("free.cpp"), &free).unwrap(),
                content: free,
                cpp_abi: None,
            },
            LoadedCarrier {
                path: PathBuf::from("method.cpp"),
                blocks: parse_blocks(Path::new("method.cpp"), &method_same_module).unwrap(),
                content: method_same_module,
                cpp_abi: None,
            },
        ];
        let error = prepare_cpp_abi_carriers(&mut carriers).expect_err("helper collision");
        assert!(error.contains("generated helper") && error.contains("_A_b"), "{error}");
    }

    #[test]
    fn cpp_abi_cross_carrier_rejects_provider_import_aliases() {
        let provider = cpp_abi_carrier(&[r#"
pub mod provider {
    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
    pub fn foo(v: Vec<u8>) -> Vec<u8> { v }
}
"#]);
        for (name, body) in [
            (
                "item",
                "use crate::provider as p; pub fn call(v: Vec<u8>) -> Vec<u8> { p::foo(v) }",
            ),
            (
                "group",
                "use crate::{provider as p}; pub fn call(v: Vec<u8>) -> Vec<u8> { p::foo(v) }",
            ),
            (
                "block-group",
                "pub fn call(v: Vec<u8>) -> Vec<u8> { use crate::{provider as p}; p::foo(v) }",
            ),
        ] {
            let consumer = cpp_abi_carrier(&[body]).replacen(
                "export module demo;",
                &format!("export module consumer_{name};"),
                1,
            );
            let mut carriers = [
                LoadedCarrier {
                    path: PathBuf::from("provider.cpp"),
                    blocks: parse_blocks(Path::new("provider.cpp"), &provider).unwrap(),
                    content: provider.clone(),
                    cpp_abi: None,
                },
                LoadedCarrier {
                    path: PathBuf::from(format!("consumer-{name}.cpp")),
                    blocks: parse_blocks(Path::new("consumer.cpp"), &consumer).unwrap(),
                    content: consumer,
                    cpp_abi: None,
                },
            ];
            let error = prepare_cpp_abi_carriers(&mut carriers)
                .expect_err("provider namespace import alias must not mask cross-carrier call");
            assert!(error.contains("adapted sibling"), "{error}");
        }

        let local = cpp_abi_carrier(&[r#"
pub mod local { pub fn foo(v: Vec<u8>) -> Vec<u8> { v } }
use crate::local as p;
pub fn call(v: Vec<u8>) -> Vec<u8> { p::foo(v) }
"#])
        .replacen("export module demo;", "export module local_control;", 1);
        let mut carriers = [
            LoadedCarrier {
                path: PathBuf::from("provider.cpp"),
                blocks: parse_blocks(Path::new("provider.cpp"), &provider).unwrap(),
                content: provider,
                cpp_abi: None,
            },
            LoadedCarrier {
                path: PathBuf::from("local-control.cpp"),
                blocks: parse_blocks(Path::new("local-control.cpp"), &local).unwrap(),
                content: local,
                cpp_abi: None,
            },
        ];
        prepare_cpp_abi_carriers(&mut carriers)
            .expect("an alias of a local module with the same function tail is unrelated");
    }

    #[test]
    fn cpp_abi_host_prerequisites_must_be_unconditional_real_statements() {
        let adapted = r#"
#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
pub fn adapted(v: Vec<u8>) {}
"#;
        for (name, directive) in [
            ("if-space", "#if 0"),
            ("if-paren", "#if(0)"),
            ("hash-space", "# if 0"),
            ("leading-comment", "/* host */ #if 0"),
        ] {
            let conditional = cpp_abi_carrier(&[adapted]).replacen(
                "import std;",
                &format!("{directive}\nimport std;\nimport rusty;\n#endif"),
                1,
            );
            let path = PathBuf::from(format!("conditional-{name}.cpp"));
            let mut carriers = [LoadedCarrier {
                blocks: parse_blocks(&path, &conditional).unwrap(),
                path,
                content: conditional,
                cpp_abi: None,
            }];
            let error =
                prepare_cpp_abi_carriers(&mut carriers).expect_err("inactive imports");
            assert!(error.contains("prior exact `import std;`"), "{error}");
        }

        for (name, directive) in [
            ("digraph", "%:if 0"),
            ("split-hash", "#\\\nif 0"),
            ("split-keyword", "#i\\\nf 0"),
        ] {
            let conditional = cpp_abi_carrier(&[adapted]).replacen(
                "import std;",
                &format!("{directive}\nimport std;\nimport rusty;\n#endif"),
                1,
            );
            let path = PathBuf::from(format!("unsupported-{name}.cpp"));
            let mut carriers = [LoadedCarrier {
                blocks: parse_blocks(&path, &conditional).unwrap(),
                path,
                content: conditional,
                cpp_abi: None,
            }];
            let error = prepare_cpp_abi_carriers(&mut carriers)
                .expect_err("unsupported directive spelling must fail closed");
            assert!(
                error.contains("line continuations")
                    || error.contains("`%:`"),
                "{error}"
            );
        }

        let multiline_comment = cpp_abi_carrier(&[adapted]).replacen(
            "import std;",
            "/*\n*/ #if 0\nimport std;\nimport rusty;\n#endif",
            1,
        );
        let path = PathBuf::from("multiline-comment-directive.cpp");
        let mut carriers = [LoadedCarrier {
            blocks: parse_blocks(&path, &multiline_comment).unwrap(),
            path,
            content: multiline_comment,
            cpp_abi: None,
        }];
        let error = prepare_cpp_abi_carriers(&mut carriers)
            .expect_err("directive after multiline comment must remain a real directive");
        assert!(error.contains("prior exact `import std;`"), "{error}");

        let inactive_brace = cpp_abi_carrier(&[adapted]).replacen(
            "export namespace rrr {",
            "#if 0\n{\n#endif\nexport namespace rrr {",
            1,
        );
        let path = PathBuf::from("inactive-brace.cpp");
        let mut carriers = [LoadedCarrier {
            blocks: parse_blocks(&path, &inactive_brace).unwrap(),
            path,
            content: inactive_brace,
            cpp_abi: None,
        }];
        prepare_cpp_abi_carriers(&mut carriers)
            .expect("inactive unmatched braces must not poison host scope");

        let balanced_conditional = cpp_abi_carrier(&[adapted]).replacen(
            "export namespace rrr {",
            "#if FEATURE\nnamespace conditional_scope {\n}\n#endif\nexport namespace rrr {",
            1,
        );
        let path = PathBuf::from("balanced-conditional.cpp");
        let mut carriers = [LoadedCarrier {
            blocks: parse_blocks(&path, &balanced_conditional).unwrap(),
            path,
            content: balanced_conditional,
            cpp_abi: None,
        }];
        prepare_cpp_abi_carriers(&mut carriers)
            .expect("a fully balanced conditional scope cannot change provider scope");

        let crossing_conditional = cpp_abi_carrier(&[adapted])
            .replacen(
                "export namespace rrr {",
                "#if FEATURE\nnamespace outer {\n#endif\nexport namespace rrr {",
                1,
            )
            .replacen(
                "} // namespace rrr",
                "} // namespace rrr\n#if FEATURE\n}\n#endif",
                1,
            );
        let path = PathBuf::from("crossing-conditional.cpp");
        let mut carriers = [LoadedCarrier {
            blocks: parse_blocks(&path, &crossing_conditional).unwrap(),
            path,
            content: crossing_conditional,
            cpp_abi: None,
        }];
        let error = prepare_cpp_abi_carriers(&mut carriers)
            .expect_err("conditional scope may not cross into the provider's host scope");
        assert!(error.contains("neutral brace scope"), "{error}");

        let macros = cpp_abi_carrier(&[adapted])
            .replacen(
                "#include <rusty/rusty.hpp>",
                "#define FAKE_RUSTY import rusty;",
                1,
            )
            .replacen(
                "export module demo;",
                "#define FAKE_MODULE export module demo;",
                1,
            )
            .replacen("import std;", "#define FAKE_STD import std;", 1);
        let path = PathBuf::from("macros.cpp");
        let mut carriers = [LoadedCarrier {
            blocks: parse_blocks(&path, &macros).unwrap(),
            path,
            content: macros,
            cpp_abi: None,
        }];
        let error = prepare_cpp_abi_carriers(&mut carriers).expect_err("macro replacements");
        assert!(error.contains("exactly one C++ module identity"), "{error}");

        let commented_directives = cpp_abi_carrier(&[adapted])
            .replacen(
                "#include <rusty/rusty.hpp>",
                "/*\n*/ #include <rusty/rusty.hpp>",
                1,
            )
            .replacen(
                "export module demo;",
                "/*\n*/ #define NOISE export module fake;\nexport module demo;",
                1,
            );
        let path = PathBuf::from("commented-directives.cpp");
        let mut carriers = [LoadedCarrier {
            blocks: parse_blocks(&path, &commented_directives).unwrap(),
            path,
            content: commented_directives,
            cpp_abi: None,
        }];
        prepare_cpp_abi_carriers(&mut carriers)
            .expect("stateful phase-3 comment removal must preserve real directives only");

        let missing_gmf = cpp_abi_carrier(&[adapted]).replacen("module;\n", "", 1);
        let path = PathBuf::from("missing-gmf.cpp");
        let mut carriers = [LoadedCarrier {
            blocks: parse_blocks(&path, &missing_gmf).unwrap(),
            path,
            content: missing_gmf,
            cpp_abi: None,
        }];
        let error = prepare_cpp_abi_carriers(&mut carriers).expect_err("missing GMF");
        assert!(error.contains("global module fragment"), "{error}");

        let late_header = cpp_abi_carrier(&[adapted])
            .replacen("#include <rusty/rusty.hpp>\n", "", 1)
            .replacen(
                "export module demo;",
                "export module demo;\n#include <rusty/rusty.hpp>",
                1,
            );
        let path = PathBuf::from("late-header.cpp");
        let mut carriers = [LoadedCarrier {
            blocks: parse_blocks(&path, &late_header).unwrap(),
            path,
            content: late_header,
            cpp_abi: None,
        }];
        let error = prepare_cpp_abi_carriers(&mut carriers).expect_err("late header");
        assert!(error.contains("global module fragment"), "{error}");

        for (name, header) in [
            ("split-line", "#include <rusty\n/rusty.hpp>"),
            ("spaced", "#include <rusty / rusty.hpp>"),
            ("suffix", "#include <rusty/rusty.hpp> junk"),
        ] {
            let malformed =
                cpp_abi_carrier(&[adapted]).replacen("#include <rusty/rusty.hpp>", header, 1);
            let path = PathBuf::from(format!("malformed-header-{name}.cpp"));
            let mut carriers = [LoadedCarrier {
                blocks: parse_blocks(&path, &malformed).unwrap(),
                path,
                content: malformed,
                cpp_abi: None,
            }];
            let error = prepare_cpp_abi_carriers(&mut carriers)
                .expect_err("only one exact directive-local rusty header is accepted");
            assert!(error.contains("global module fragment"), "{error}");
        }

        let declaration_before_gmf = cpp_abi_carrier(&[adapted])
            .replacen("module;", "int host_value;\nmodule;", 1);
        let path = PathBuf::from("declaration-before-gmf.cpp");
        let mut carriers = [LoadedCarrier {
            blocks: parse_blocks(&path, &declaration_before_gmf).unwrap(),
            path,
            content: declaration_before_gmf,
            cpp_abi: None,
        }];
        let error = prepare_cpp_abi_carriers(&mut carriers).expect_err("GMF must be first");
        assert!(error.contains("global module fragment"), "{error}");

        let nested_header = cpp_abi_carrier(&[adapted])
            .replacen(
                "#include <rusty/rusty.hpp>",
                "namespace host_detail {\n#include <rusty/rusty.hpp>\n}",
                1,
            );
        let path = PathBuf::from("nested-header.cpp");
        let mut carriers = [LoadedCarrier {
            blocks: parse_blocks(&path, &nested_header).unwrap(),
            path,
            content: nested_header,
            cpp_abi: None,
        }];
        let error = prepare_cpp_abi_carriers(&mut carriers)
            .expect_err("header inside a namespace is not a GMF include");
        assert!(error.contains("global module fragment"), "{error}");

        let header_only = cpp_abi_carrier(&[adapted]).replacen("import rusty;\n", "", 1);
        let path = PathBuf::from("header-only.cpp");
        let mut carriers = [LoadedCarrier {
            blocks: parse_blocks(&path, &header_only).unwrap(),
            path,
            content: header_only,
            cpp_abi: None,
        }];
        let error = prepare_cpp_abi_carriers(&mut carriers)
            .expect_err("the GMF header cannot substitute for the rusty module import");
        assert!(error.contains("prior exact `import rusty;`"), "{error}");

        let import_mode = cpp_abi_carrier(&[adapted])
            .replacen("#include <rusty/rusty.hpp>", "", 1);
        let path = PathBuf::from("import-mode.cpp");
        let mut carriers = [LoadedCarrier {
            blocks: parse_blocks(&path, &import_mode).unwrap(),
            path,
            content: import_mode,
            cpp_abi: None,
        }];
        prepare_cpp_abi_carriers(&mut carriers)
            .expect("ordinary module imports after export module satisfy host prerequisites");

        for (name, import) in [("std", "import std;"), ("rusty", "import rusty;")] {
            let mut early_import = cpp_abi_carrier(&[adapted]);
            if name == "std" {
                early_import = early_import.replacen("import std;\n", "", 1);
            } else {
                early_import = early_import.replacen("import rusty;\n", "", 1);
            }
            early_import = early_import.replacen(
                "export module demo;",
                &format!("{import}\nexport module demo;"),
                1,
            );
            let path = PathBuf::from(format!("early-{name}.cpp"));
            let mut carriers = [LoadedCarrier {
                blocks: parse_blocks(&path, &early_import).unwrap(),
                path,
                content: early_import,
                cpp_abi: None,
            }];
            let error = prepare_cpp_abi_carriers(&mut carriers)
                .expect_err("imports before the module declaration must not satisfy ABI host needs");
            assert!(
                error.contains("requires prior") || error.contains("requires a prior"),
                "{error}"
            );
        }
    }

    #[test]
    fn cpp_abi_identity_is_batch_stable_and_failures_do_not_mutate_any_carrier() {
        let temp = tempfile::tempdir().unwrap();
        let single_dir = temp.path().join("single");
        let batch_dir = temp.path().join("batch");
        std::fs::create_dir_all(&single_dir).unwrap();
        std::fs::create_dir_all(&batch_dir).unwrap();
        let source_a = cpp_abi_carrier(&[r#"
#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
pub fn bytes_a(v: Vec<u8>) -> Vec<u8> { v }
"#]);
        let source_b = cpp_abi_carrier(&[r#"
#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
pub fn bytes_b(v: Vec<u8>) -> Vec<u8> { v }
"#])
        .replacen("export module demo;", "export module demo_b;", 1);
        let single_a = single_dir.join("a.cpp");
        let batch_a = batch_dir.join("a.cpp");
        let batch_b = batch_dir.join("b.cpp");
        std::fs::write(&single_a, &source_a).unwrap();
        std::fs::write(&batch_a, &source_a).unwrap();
        std::fs::write(&batch_b, &source_b).unwrap();
        run_inline_rust(&InlineRustOptions {
            mode: InlineRustMode::Rewrite,
            files: vec![single_a.clone()],
        })
        .unwrap();
        run_inline_rust(&InlineRustOptions {
            mode: InlineRustMode::Rewrite,
            files: vec![batch_a.clone(), batch_b],
        })
        .unwrap();
        assert_eq!(
            std::fs::read(&single_a).unwrap(),
            std::fs::read(&batch_a).unwrap(),
            "module-derived identities must not depend on invocation batching"
        );

        let first = temp.path().join("sentinel-first.cpp");
        let bad = temp.path().join("bad.cpp");
        let first_source = source_a.replacen("export module demo;", "export module first;", 1);
        let bad_source = cpp_abi_carrier(&[
            "pub fn before(v: Vec<u8>) -> Vec<u8> { later(v) }\n",
            r#"
#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
pub fn later(v: Vec<u8>) -> Vec<u8> { v }
"#,
        ])
        .replacen("export module demo;", "export module bad;", 1);
        std::fs::write(&first, &first_source).unwrap();
        std::fs::write(&bad, &bad_source).unwrap();
        let error = run_inline_rust(&InlineRustOptions {
            mode: InlineRustMode::Rewrite,
            files: vec![first.clone(), bad],
        })
        .expect_err("later-provider failure must abort the batch");
        assert!(error.contains("later"), "{error}");
        assert_eq!(std::fs::read_to_string(first).unwrap(), first_source);
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
    fn test_inline_cpp_inherit_uses_cargo_identity_across_blocks_and_rejects_lookalike() {
        fn write(path: &Path, contents: &str) {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        }

        let fixture = tempfile::tempdir().unwrap();
        let macros = fixture.path().join("rusty_macros");
        let runtime = fixture.path().join("rusty");
        let genuine = fixture.path().join("genuine");
        write(
            &macros.join("Cargo.toml"),
            "[package]\nname='rusty_macros'\nversion='0.0.0'\nedition='2024'\n[lib]\nproc-macro=true\n[workspace]\n",
        );
        write(
            &macros.join("src/lib.rs"),
            r#"use proc_macro::TokenStream;
#[proc_macro_attribute]
pub fn cpp_inherit(_: TokenStream, item: TokenStream) -> TokenStream { item }
"#,
        );
        write(
            &runtime.join("Cargo.toml"),
            "[package]\nname='rusty'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nrusty_macros={path='../rusty_macros'}\n[workspace]\n",
        );
        write(
            &runtime.join("src/lib.rs"),
            "pub use rusty_macros::cpp_inherit;\n",
        );
        write(
            &genuine.join("Cargo.toml"),
            "[package]\nname='genuine_inline'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nrusty={path='../rusty'}\n[workspace]\n",
        );
        let rust_source = r#"use rusty::cpp_inherit;
pub trait Base { fn value(&self) -> i32; }
pub struct Derived { pub value: i32 }
#[cpp_inherit]
impl Base for Derived { fn value(&self) -> i32 { self.value } }
"#;
        write(&genuine.join("src/lib.rs"), rust_source);
        let cargo_check = std::process::Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(genuine.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", fixture.path().join("target-genuine"))
            .output()
            .unwrap();
        assert!(
            cargo_check.status.success(),
            "genuine inline fixture must be Cargo-valid:\n{}",
            String::from_utf8_lossy(&cargo_check.stderr)
        );

        let carrier = genuine.join("carrier.cpp");
        write(
            &carrier,
            r#"#if RUSTYCPP_RUST
use rusty::cpp_inherit;
#endif
#if RUSTYCPP_RUST
trait Base { fn value(&self) -> i32; }
struct Derived { value: i32 }
#[cpp_inherit]
impl Base for Derived { fn value(&self) -> i32 { self.value } }
#endif
"#,
        );
        run_inline_rust(&InlineRustOptions {
            mode: InlineRustMode::Rewrite,
            files: vec![carrier.clone()],
        })
        .unwrap();
        let rewritten = std::fs::read_to_string(&carrier).unwrap();
        assert!(
            rewritten.contains("struct Derived : public Base"),
            "a genuine marker imported in a sibling block lost direct inheritance:\n{rewritten}"
        );
        assert!(
            !rewritten.contains("class BaseAdapter<Derived>"),
            "genuine inline marker fell back to an Adapter:\n{rewritten}"
        );
        run_inline_rust(&InlineRustOptions {
            mode: InlineRustMode::Check,
            files: vec![carrier],
        })
        .unwrap();

        let lookalike = fixture.path().join("lookalike");
        write(
            &lookalike.join("Cargo.toml"),
            "[package]\nname='lookalike_inline'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nevil={package='rusty_macros',path='../rusty_macros'}\n[workspace]\n",
        );
        let lookalike_rust = r#"mod rusty { pub use evil::cpp_inherit; }
use rusty::cpp_inherit;
pub trait Base { fn value(&self) -> i32; }
pub struct Derived { pub value: i32 }
#[cpp_inherit]
impl Base for Derived { fn value(&self) -> i32 { self.value } }
"#;
        write(&lookalike.join("src/lib.rs"), lookalike_rust);
        let cargo_check = std::process::Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(lookalike.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", fixture.path().join("target-lookalike"))
            .output()
            .unwrap();
        assert!(
            cargo_check.status.success(),
            "lookalike inline fixture must be Cargo-valid:\n{}",
            String::from_utf8_lossy(&cargo_check.stderr)
        );
        let lookalike_carrier = lookalike.join("carrier.cpp");
        write(
            &lookalike_carrier,
            &format!("#if RUSTYCPP_RUST\n{lookalike_rust}#endif\n"),
        );
        run_inline_rust(&InlineRustOptions {
            mode: InlineRustMode::Rewrite,
            files: vec![lookalike_carrier.clone()],
        })
        .unwrap();
        let rewritten = std::fs::read_to_string(lookalike_carrier).unwrap();
        assert!(
            !rewritten.contains("struct Derived : public Base")
                && rewritten.contains("class BaseAdapter<Derived>"),
            "a local lookalike acquired compiler-owned inheritance:\n{rewritten}"
        );

        let unowned = fixture.path().join("unowned.cpp");
        write(
            &unowned,
            r#"#if RUSTYCPP_RUST
use rusty::cpp_inherit;
trait Base { fn value(&self) -> i32; }
struct Derived { value: i32 }
#[cpp_inherit]
impl Base for Derived { fn value(&self) -> i32 { self.value } }
#endif
"#,
        );
        let original = std::fs::read_to_string(&unowned).unwrap();
        let error = run_inline_rust(&InlineRustOptions {
            mode: InlineRustMode::Rewrite,
            files: vec![unowned.clone()],
        })
        .expect_err("a compiler marker without Cargo provenance must fail loudly");
        assert!(error.contains("requires a Cargo manifest"), "{error}");
        assert_eq!(std::fs::read_to_string(unowned).unwrap(), original);
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


#[cfg(test)]
mod cpp_alias_tests {
    use super::collect_cpp_type_aliases;

    #[test]
    fn scans_using_alias_declarations() {
        let src = r#"
namespace rrr {
using WeakClientConnection = rusty::sync::Weak<ClientConnection>;
using OnFrameCallback = detail::CallbackWrapper<void(const ChannelFrame&) const>;
using namespace Serialize_;
using rusty::sync::atomic::Ordering;
}
"#;
        let m = collect_cpp_type_aliases(src);
        assert_eq!(
            m.get("WeakClientConnection").map(String::as_str),
            Some("rusty::sync::Weak<ClientConnection>")
        );
        assert!(m.contains_key("OnFrameCallback"));
        // `using namespace N;` and using-DECLARATIONS have no `=` and must
        // not be mistaken for aliases.
        assert!(!m.contains_key("namespace"));
        assert_eq!(m.len(), 2, "only the two `using X = Y;` forms: {m:?}");
    }
}
