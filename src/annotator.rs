use crate::parser;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct AnnotationSummary {
    pub safe_count: usize,
    pub unsafe_count: usize,
    pub skipped_count: usize,
}

#[derive(Debug, Clone)]
struct CandidateFunction {
    name: String,
    line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnnotationMode {
    Safe,
    Unsafe,
}

impl AnnotationMode {
    fn as_comment(self) -> &'static str {
        match self {
            AnnotationMode::Safe => "// @safe",
            AnnotationMode::Unsafe => "// @unsafe",
        }
    }
}

pub fn annotate_file(
    path: &PathBuf,
    include_paths: &[PathBuf],
    defines: &[String],
    compile_commands: Option<&PathBuf>,
) -> Result<AnnotationSummary, String> {
    let candidates = collect_candidates(path, include_paths, defines, compile_commands)?;
    if candidates.functions.is_empty() {
        return Ok(AnnotationSummary {
            skipped_count: candidates.skipped_count,
            ..AnnotationSummary::default()
        });
    }

    let mut modes = candidates
        .functions
        .iter()
        .map(|function| (function.line, AnnotationMode::Safe))
        .collect::<BTreeMap<_, _>>();
    let candidate_names = candidates
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<HashSet<_>>();

    loop {
        let temp_path = write_annotated_temp_file(path, &candidates.original_lines, &modes)?;
        let analysis_result =
            crate::analyze_file(&temp_path, include_paths, defines, compile_commands);
        let _ = fs::remove_file(&temp_path);

        let violations = analysis_result?;
        let violating_names = extract_violating_function_names(&violations, &candidate_names);
        let mut changed = false;

        for function in &candidates.functions {
            if violating_names.contains(&function.name)
                && modes.get(&function.line) != Some(&AnnotationMode::Unsafe)
            {
                modes.insert(function.line, AnnotationMode::Unsafe);
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    let final_source = render_with_annotations(&candidates.original_lines, &modes);
    fs::write(path, final_source)
        .map_err(|e| format!("Failed to write annotations to {}: {}", path.display(), e))?;

    let safe_count = modes
        .values()
        .filter(|&&mode| mode == AnnotationMode::Safe)
        .count();
    let unsafe_count = modes.len() - safe_count;

    Ok(AnnotationSummary {
        safe_count,
        unsafe_count,
        skipped_count: candidates.skipped_count,
    })
}

struct CandidateCollection {
    original_lines: Vec<String>,
    functions: Vec<CandidateFunction>,
    skipped_count: usize,
}

fn collect_candidates(
    path: &PathBuf,
    include_paths: &[PathBuf],
    defines: &[String],
    compile_commands: Option<&PathBuf>,
) -> Result<CandidateCollection, String> {
    let analysis_config = crate::build_analysis_config(path, include_paths, compile_commands)?;
    let ast = parser::parse_cpp_file_with_includes_defines_and_args(
        path,
        &analysis_config.include_paths,
        defines,
        &analysis_config.extra_clang_args,
    )?;
    let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
    let original = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let original_lines = original
        .lines()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    let mut functions = Vec::new();
    let mut skipped_count = 0usize;
    let mut seen_lines = BTreeSet::new();

    let parsed_functions = ast
        .functions
        .iter()
        .chain(ast.classes.iter().flat_map(|class| class.methods.iter()));

    for function in parsed_functions {
        let fn_path = fs::canonicalize(&function.location.file)
            .unwrap_or_else(|_| PathBuf::from(&function.location.file));
        if fn_path != canonical_path {
            continue;
        }

        if function.has_explicit_safety_annotation {
            skipped_count += 1;
            continue;
        }

        let line = function.location.line as usize;
        if line == 0 || line > original_lines.len() {
            continue;
        }

        if has_existing_annotation(&original_lines, line) {
            skipped_count += 1;
            continue;
        }

        if seen_lines.insert(line) {
            functions.push(CandidateFunction {
                name: function.name.clone(),
                line,
            });
        }
    }

    functions.sort_by_key(|function| function.line);

    Ok(CandidateCollection {
        original_lines,
        functions,
        skipped_count,
    })
}

fn has_existing_annotation(lines: &[String], one_based_line: usize) -> bool {
    let mut idx = one_based_line.saturating_sub(1);
    while idx > 0 {
        idx -= 1;
        let trimmed = lines[idx].trim();
        if trimmed.is_empty() {
            continue;
        }
        return is_safety_annotation(trimmed);
    }
    false
}

fn is_safety_annotation(line: &str) -> bool {
    line.starts_with("// @safe") || line.starts_with("// @unsafe") || line.starts_with("// @bridge")
}

fn write_annotated_temp_file(
    original_path: &Path,
    lines: &[String],
    modes: &BTreeMap<usize, AnnotationMode>,
) -> Result<PathBuf, String> {
    let file_name = original_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("source.cpp");
    let temp_path = original_path.with_file_name(format!(".{}.rusty-annotator.tmp.cpp", file_name));
    fs::write(&temp_path, render_with_annotations(lines, modes)).map_err(|e| {
        format!(
            "Failed to write temporary annotated file {}: {}",
            temp_path.display(),
            e
        )
    })?;
    Ok(temp_path)
}

fn render_with_annotations(lines: &[String], modes: &BTreeMap<usize, AnnotationMode>) -> String {
    let mut rendered = String::new();
    for (idx, line) in lines.iter().enumerate() {
        let one_based_line = idx + 1;
        if let Some(mode) = modes.get(&one_based_line) {
            rendered.push_str(mode.as_comment());
            rendered.push('\n');
        }
        rendered.push_str(line);
        rendered.push('\n');
    }
    rendered
}

fn extract_violating_function_names(
    violations: &[String],
    candidate_names: &HashSet<String>,
) -> HashSet<String> {
    let re = Regex::new(r"In function '([^']+)'").expect("valid function diagnostic regex");
    let mut names = HashSet::new();

    for violation in violations {
        for captures in re.captures_iter(violation) {
            if let Some(name) = captures.get(1).map(|m| m.as_str()) {
                if candidate_names.contains(name) {
                    names.insert(name.to_string());
                    continue;
                }

                if let Some(candidate_name) = candidate_names.iter().find(|candidate| {
                    candidate.ends_with(&format!("::{}", name))
                        || name.ends_with(&format!("::{}", candidate))
                }) {
                    names.insert(candidate_name.clone());
                }
            }
        }
    }

    names
}
