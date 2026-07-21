//! Safe compilation-database discovery and argv normalization.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use whyvec_experiment::{process_request, run_process};

use crate::ParameterCandidate;

const MAX_RESPONSE_DEPTH: usize = 8;
const MAX_RESPONSE_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResponseFileFingerprint {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompilationCommand {
    pub database_path: String,
    pub database_digest: String,
    pub entry_digest: String,
    pub directory: String,
    pub source: String,
    pub compiler: String,
    pub arguments: Vec<String>,
    pub response_files: Vec<ResponseFileFingerprint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceMapping {
    pub function: String,
    pub candidates: Vec<ParameterCandidate>,
}

#[derive(Debug)]
pub enum CompilationDatabaseError {
    Io(std::io::Error),
    Json(serde_json::Error),
    MissingDatabase,
    MissingEntry,
    AmbiguousEntry(usize),
    InvalidEntry(String),
    PolicyDenied(String),
    MappingDeclined(String),
}

impl std::fmt::Display for CompilationDatabaseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::Json(error) => error.fmt(formatter),
            Self::MissingDatabase => formatter.write_str("compilation database not found"),
            Self::MissingEntry => formatter.write_str("source has no compilation database entry"),
            Self::AmbiguousEntry(count) => {
                write!(formatter, "source has {count} compilation database entries")
            }
            Self::InvalidEntry(detail) => write!(formatter, "invalid compilation entry: {detail}"),
            Self::PolicyDenied(detail) => write!(formatter, "compilation entry denied: {detail}"),
            Self::MappingDeclined(detail) => write!(formatter, "source mapping declined: {detail}"),
        }
    }
}

impl std::error::Error for CompilationDatabaseError {}

impl From<std::io::Error> for CompilationDatabaseError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for CompilationDatabaseError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RawEntry {
    directory: String,
    file: String,
    #[serde(default)]
    arguments: Option<Vec<String>>,
    #[serde(default)]
    command: Option<String>,
}

/// Resolves exactly one safe compilation entry for `source`.
///
/// Discovery is bounded to the repository root and three directory levels,
/// with build-output names preferred by ordinary CMake/Ninja layouts.
///
/// # Errors
///
/// Returns a typed missing, ambiguous, malformed, policy, or filesystem error.
pub fn resolve_compilation_command(
    repository: &Path,
    source: &Path,
) -> Result<CompilationCommand, CompilationDatabaseError> {
    let repository = repository.canonicalize()?;
    let source = source.canonicalize()?;
    if !source.starts_with(&repository) {
        return Err(CompilationDatabaseError::PolicyDenied(
            "selected source escapes repository".to_owned(),
        ));
    }
    let databases = discover_databases(&repository)?;
    if databases.is_empty() {
        return Err(CompilationDatabaseError::MissingDatabase);
    }
    let mut matches = Vec::new();
    for database in databases {
        let bytes = fs::read(&database)?;
        let entries: Vec<RawEntry> = serde_json::from_slice(&bytes)?;
        for entry in entries {
            let directory =
                absolute_from(database.parent().unwrap_or(&repository), &entry.directory)?;
            let entry_source = absolute_from(&directory, &entry.file)?;
            if entry_source == source {
                matches.push((database.clone(), bytes.clone(), directory, entry));
            }
        }
    }
    match matches.len() {
        0 => Err(CompilationDatabaseError::MissingEntry),
        1 => normalize_entry(
            &repository,
            &source,
            matches.pop().ok_or_else(|| {
                CompilationDatabaseError::InvalidEntry("selected entry disappeared".to_owned())
            })?,
        ),
        count => Err(CompilationDatabaseError::AmbiguousEntry(count)),
    }
}

/// Infers the containing C function and pointer-parameter-to-IR mapping.
///
/// This first automatic mapping surface intentionally supports direct C
/// functions only. C++ ABI lowering remains an expert path because hidden
/// parameters and method receivers require ABI-aware mapping.
///
/// # Errors
///
/// Returns a typed decline when Clang cannot produce a unique supported C
/// function/loop mapping, or when AST extraction fails safely.
pub fn infer_c_source_mapping(
    compilation: &CompilationCommand,
    source: &Path,
    line: u64,
) -> Result<SourceMapping, CompilationDatabaseError> {
    if source.extension().and_then(|value| value.to_str()) != Some("c") {
        return Err(CompilationDatabaseError::MappingDeclined(
            "automatic parameter mapping currently supports C translation units only".to_owned(),
        ));
    }
    let mut arguments = compilation.arguments.clone();
    arguments.extend([
        "-Xclang".to_owned(),
        "-ast-dump=json".to_owned(),
        "-fsyntax-only".to_owned(),
        source.to_string_lossy().into_owned(),
    ]);
    let mut request = process_request(
        Path::new(&compilation.compiler),
        &arguments,
        Path::new(&compilation.directory),
    );
    request.timeout = std::time::Duration::from_secs(90);
    request.output_limit = 32 * 1024 * 1024;
    let result = run_process(&request).map_err(|error| {
        CompilationDatabaseError::MappingDeclined(format!("Clang AST extraction failed: {error}"))
    })?;
    if result.exit_code != Some(0)
        || result.timed_out
        || result.stdout_truncated
        || result.stderr_truncated
    {
        return Err(CompilationDatabaseError::MappingDeclined(format!(
            "Clang AST extraction failed with exit {:?}: {}",
            result.exit_code,
            String::from_utf8_lossy(&result.stderr).trim()
        )));
    }
    let ast: JsonValue = serde_json::from_slice(&result.stdout)?;
    mapping_from_ast(&ast, line)
}

fn mapping_from_ast(ast: &JsonValue, line: u64) -> Result<SourceMapping, CompilationDatabaseError> {
    let mut matches = Vec::new();
    collect_function_mappings(ast, line, &mut matches);
    match matches.len() {
        0 => Err(CompilationDatabaseError::MappingDeclined(
            "no direct C function contains a loop at the selected line".to_owned(),
        )),
        1 => Ok(matches.pop().expect("one mapping")),
        count => Err(CompilationDatabaseError::MappingDeclined(format!(
            "{count} functions contain loops at the selected line"
        ))),
    }
}

fn collect_function_mappings(node: &JsonValue, line: u64, matches: &mut Vec<SourceMapping>) {
    if node.get("kind").and_then(JsonValue::as_str) == Some("FunctionDecl")
        && contains_selected_loop(node, line)
    {
        let Some(function) = node.get("name").and_then(JsonValue::as_str) else {
            return;
        };
        let candidates = node
            .get("inner")
            .and_then(JsonValue::as_array)
            .into_iter()
            .flatten()
            .filter(|child| child.get("kind").and_then(JsonValue::as_str) == Some("ParmVarDecl"))
            .enumerate()
            .filter_map(|(index, parameter)| {
                let name = parameter.get("name")?.as_str()?;
                let qualified = parameter.get("type")?.get("qualType")?.as_str()?;
                qualified.contains('*').then(|| ParameterCandidate {
                    source_name: name.to_owned(),
                    ir_index: index,
                })
            })
            .collect::<Vec<_>>();
        if !candidates.is_empty() {
            matches.push(SourceMapping {
                function: function.to_owned(),
                candidates,
            });
        }
        return;
    }
    if let Some(children) = node.get("inner").and_then(JsonValue::as_array) {
        for child in children {
            collect_function_mappings(child, line, matches);
        }
    }
}

fn contains_selected_loop(node: &JsonValue, line: u64) -> bool {
    if node.get("kind").and_then(JsonValue::as_str) == Some("ForStmt")
        && source_line(node) == Some(line)
    {
        return true;
    }
    node.get("inner")
        .and_then(JsonValue::as_array)
        .is_some_and(|children| {
            children
                .iter()
                .any(|child| contains_selected_loop(child, line))
        })
}

fn source_line(node: &JsonValue) -> Option<u64> {
    node.get("loc")
        .and_then(|loc| loc.get("line"))
        .or_else(|| node.get("range")?.get("begin")?.get("line"))
        .and_then(JsonValue::as_u64)
}

fn discover_databases(repository: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    fn visit(
        root: &Path,
        directory: &Path,
        depth: usize,
        found: &mut Vec<PathBuf>,
    ) -> Result<(), std::io::Error> {
        if depth > 3 {
            return Ok(());
        }
        let candidate = directory.join("compile_commands.json");
        if candidate.is_file() {
            found.push(candidate.canonicalize()?);
        }
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.')
                || matches!(
                    name.as_ref(),
                    "target" | "vendor" | "node_modules" | "evidence"
                )
            {
                continue;
            }
            if path.starts_with(root) {
                visit(root, &path, depth + 1, found)?;
            }
        }
        Ok(())
    }
    let mut found = Vec::new();
    visit(repository, repository, 0, &mut found)?;
    found.sort();
    found.dedup();
    Ok(found)
}

fn normalize_entry(
    repository: &Path,
    source: &Path,
    (database, database_bytes, directory, entry): (PathBuf, Vec<u8>, PathBuf, RawEntry),
) -> Result<CompilationCommand, CompilationDatabaseError> {
    let mut argv = match (entry.arguments, entry.command) {
        (Some(arguments), None) => arguments,
        (None, Some(command)) => shlex::split(&command).ok_or_else(|| {
            CompilationDatabaseError::InvalidEntry("command has invalid quoting".to_owned())
        })?,
        _ => {
            return Err(CompilationDatabaseError::InvalidEntry(
                "entry must contain exactly one of arguments or command".to_owned(),
            ));
        }
    };
    if argv.is_empty() {
        return Err(CompilationDatabaseError::InvalidEntry(
            "empty argv".to_owned(),
        ));
    }
    reject_unsafe_compiler(&argv[0])?;
    let compiler = absolute_from(&directory, argv.remove(0))?;
    let compiler_name = compiler
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !is_clang_frontend(compiler_name) {
        return Err(CompilationDatabaseError::PolicyDenied(format!(
            "unsupported compiler frontend {compiler_name:?}"
        )));
    }
    let mut response_files = Vec::new();
    let mut active = BTreeSet::new();
    let expanded = expand_arguments(
        repository,
        &directory,
        argv,
        0,
        &mut active,
        &mut response_files,
    )?;
    reject_unsafe_arguments(&expanded)?;
    let arguments = strip_driver_outputs(&directory, source, &expanded);
    if arguments.is_empty() {
        return Err(CompilationDatabaseError::InvalidEntry(
            "entry has no semantic frontend arguments".to_owned(),
        ));
    }
    let entry_value = serde_json::json!({
        "directory": directory,
        "source": source,
        "compiler": compiler,
        "arguments": arguments,
        "response_files": response_files,
    });
    Ok(CompilationCommand {
        database_path: database.to_string_lossy().into_owned(),
        database_digest: digest(&database_bytes),
        entry_digest: digest(&serde_json::to_vec(&entry_value)?),
        directory: directory.to_string_lossy().into_owned(),
        source: source.to_string_lossy().into_owned(),
        compiler: compiler.to_string_lossy().into_owned(),
        arguments,
        response_files,
    })
}

fn expand_arguments(
    repository: &Path,
    directory: &Path,
    arguments: Vec<String>,
    depth: usize,
    active: &mut BTreeSet<PathBuf>,
    fingerprints: &mut Vec<ResponseFileFingerprint>,
) -> Result<Vec<String>, CompilationDatabaseError> {
    if depth > MAX_RESPONSE_DEPTH {
        return Err(CompilationDatabaseError::PolicyDenied(
            "response-file nesting limit exceeded".to_owned(),
        ));
    }
    let mut expanded = Vec::new();
    for argument in arguments {
        let Some(raw_path) = argument.strip_prefix('@') else {
            expanded.push(argument);
            continue;
        };
        if raw_path.is_empty() {
            return Err(CompilationDatabaseError::InvalidEntry(
                "empty response-file path".to_owned(),
            ));
        }
        let path = absolute_from(directory, raw_path)?;
        if !path.starts_with(repository) {
            return Err(CompilationDatabaseError::PolicyDenied(
                "response file escapes repository".to_owned(),
            ));
        }
        if !active.insert(path.clone()) {
            return Err(CompilationDatabaseError::PolicyDenied(
                "response-file cycle detected".to_owned(),
            ));
        }
        let metadata = fs::metadata(&path)?;
        if metadata.len() > MAX_RESPONSE_BYTES {
            return Err(CompilationDatabaseError::PolicyDenied(
                "response file exceeds size limit".to_owned(),
            ));
        }
        let bytes = fs::read(&path)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            CompilationDatabaseError::InvalidEntry("response file is not UTF-8".to_owned())
        })?;
        let nested = shlex::split(text).ok_or_else(|| {
            CompilationDatabaseError::InvalidEntry("response file has invalid quoting".to_owned())
        })?;
        fingerprints.push(ResponseFileFingerprint {
            path: path.to_string_lossy().into_owned(),
            sha256: digest(&bytes),
            size: metadata.len(),
        });
        expanded.extend(expand_arguments(
            repository,
            directory,
            nested,
            depth + 1,
            active,
            fingerprints,
        )?);
        active.remove(&path);
    }
    Ok(expanded)
}

fn reject_unsafe_compiler(program: &str) -> Result<(), CompilationDatabaseError> {
    let name = Path::new(program)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if matches!(
        name,
        "sh" | "bash" | "zsh" | "fish" | "cmd" | "powershell" | "env" | "ccache" | "sccache"
    ) {
        return Err(CompilationDatabaseError::PolicyDenied(format!(
            "wrapper or shell {name:?} is not allowed"
        )));
    }
    Ok(())
}

fn is_clang_frontend(name: &str) -> bool {
    name == "clang"
        || name == "clang++"
        || name
            .strip_prefix("clang-")
            .is_some_and(|tail| tail.chars().all(|c| c.is_ascii_digit() || c == '.'))
        || name
            .strip_prefix("clang++-")
            .is_some_and(|tail| tail.chars().all(|c| c.is_ascii_digit() || c == '.'))
}

fn reject_unsafe_arguments(arguments: &[String]) -> Result<(), CompilationDatabaseError> {
    for (index, argument) in arguments.iter().enumerate() {
        let prior = index.checked_sub(1).and_then(|prior| arguments.get(prior));
        if matches!(argument.as_str(), "|" | "||" | "&&" | ";" | "`")
            || argument.contains("$(")
            || argument.starts_with("-fplugin")
            || argument.starts_with("-fpass-plugin")
            || argument == "-load"
            || argument == "-load-pass-plugin"
            || (argument == "-Xclang"
                && arguments.get(index + 1).is_some_and(|next| {
                    next == "-load" || next == "-plugin" || next == "-add-plugin"
                }))
            || (prior.is_some_and(|value| value == "-mllvm") && argument.starts_with("-load"))
        {
            return Err(CompilationDatabaseError::PolicyDenied(format!(
                "unsafe argument {argument:?}"
            )));
        }
    }
    Ok(())
}

fn strip_driver_outputs(directory: &Path, source: &Path, arguments: &[String]) -> Vec<String> {
    let mut kept = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        if matches!(argument.as_str(), "-o" | "-MF" | "-MT" | "-MQ" | "-MJ") {
            index += 2;
            continue;
        }
        if argument == "-c" || matches!(argument.as_str(), "-MD" | "-MMD" | "-MP" | "-MG") {
            index += 1;
            continue;
        }
        let resolved = if argument.starts_with('-') {
            None
        } else {
            absolute_from(directory, argument).ok()
        };
        if resolved.as_deref() == Some(source) {
            index += 1;
            continue;
        }
        kept.push(argument.clone());
        index += 1;
    }
    kept
}

fn absolute_from(base: &Path, path: impl AsRef<Path>) -> Result<PathBuf, std::io::Error> {
    let path = path.as_ref();
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    joined.canonicalize()
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to String cannot fail");
            output
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("whyvec-compdb-{name}-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("build")).unwrap();
        fs::write(root.join("kernel.c"), "void kernel(void) {}\n").unwrap();
        root
    }

    fn write_database(root: &Path, entries: &serde_json::Value) {
        fs::write(
            root.join("build/compile_commands.json"),
            serde_json::to_vec_pretty(entries).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn resolves_command_form_and_preserves_semantic_flags() {
        let root = fixture("flags");
        let source = root.join("kernel.c");
        write_database(
            &root,
            &serde_json::json!([{
                "directory": root,
                "file": source,
                "command": format!("/usr/bin/clang-21 -std=c17 --target=x86_64-linux-gnu -I include -DVALUE=3 -O2 -c {} -o build/kernel.o", source.display())
            }]),
        );
        let resolved = resolve_compilation_command(&root, &source).unwrap();
        assert_eq!(
            resolved.compiler,
            fs::canonicalize("/usr/bin/clang-21")
                .unwrap()
                .to_string_lossy()
        );
        assert_eq!(
            resolved.arguments,
            [
                "-std=c17",
                "--target=x86_64-linux-gnu",
                "-I",
                "include",
                "-DVALUE=3",
                "-O2"
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn expands_and_fingerprints_bounded_response_files() {
        let root = fixture("response");
        let source = root.join("kernel.c");
        fs::write(root.join("build/flags.rsp"), "-std=c11 -DANSWER=42 -O3").unwrap();
        write_database(
            &root,
            &serde_json::json!([{
                "directory": root.join("build"),
                "file": source,
                "arguments": ["/usr/bin/clang-21", "@flags.rsp", "-c", source]
            }]),
        );
        let resolved = resolve_compilation_command(&root, &source).unwrap();
        assert_eq!(resolved.arguments, ["-std=c11", "-DANSWER=42", "-O3"]);
        assert_eq!(resolved.response_files.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn declines_ambiguous_entries() {
        let root = fixture("ambiguous");
        let source = root.join("kernel.c");
        let entry = serde_json::json!({
            "directory": root,
            "file": source,
            "arguments": ["/usr/bin/clang-21", "-c", source]
        });
        write_database(&root, &serde_json::json!([entry.clone(), entry]));
        assert!(matches!(
            resolve_compilation_command(&root, &source),
            Err(CompilationDatabaseError::AmbiguousEntry(2))
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn declines_shells_plugins_and_response_escape() {
        let root = fixture("policy");
        let source = root.join("kernel.c");
        write_database(
            &root,
            &serde_json::json!([{
                "directory": root,
                "file": source,
                "arguments": ["sh", "-c", "clang kernel.c"]
            }]),
        );
        assert!(matches!(
            resolve_compilation_command(&root, &source),
            Err(CompilationDatabaseError::PolicyDenied(_))
        ));
        write_database(
            &root,
            &serde_json::json!([{
                "directory": root,
                "file": source,
                "arguments": ["/usr/bin/clang-21", "-fplugin=evil.so", "-c", source]
            }]),
        );
        assert!(matches!(
            resolve_compilation_command(&root, &source),
            Err(CompilationDatabaseError::PolicyDenied(_))
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn maps_direct_c_function_pointer_parameters_in_source_order() {
        let ast = serde_json::json!({
            "kind": "TranslationUnitDecl",
            "inner": [{
                "kind": "FunctionDecl",
                "name": "kernel",
                "inner": [
                    {"kind": "ParmVarDecl", "name": "output", "type": {"qualType": "int *"}},
                    {"kind": "ParmVarDecl", "name": "scale", "type": {"qualType": "int"}},
                    {"kind": "ParmVarDecl", "name": "count", "type": {"qualType": "const int *"}},
                    {"kind": "CompoundStmt", "inner": [{"kind": "ForStmt", "range": {"begin": {"line": 9}}}]}
                ]
            }]
        });
        let mapping = mapping_from_ast(&ast, 9).unwrap();
        assert_eq!(mapping.function, "kernel");
        assert_eq!(
            mapping.candidates,
            [
                ParameterCandidate {
                    source_name: "output".to_owned(),
                    ir_index: 0
                },
                ParameterCandidate {
                    source_name: "count".to_owned(),
                    ir_index: 2
                },
            ]
        );
    }
}
