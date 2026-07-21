use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use whyvec_experiment::{ArtifactReference, ArtifactStore};

use super::{
    OptimizationError, OptimizationReport, OptimizationTool, capture_tool, digest, execute,
    pretty_json, require_success, semantic_digest,
};

#[derive(Clone, Debug)]
pub struct GccObservationRequest {
    pub repository: PathBuf,
    pub source: PathBuf,
    pub function: String,
    pub line: u64,
    pub gcc: PathBuf,
    pub gzip: PathBuf,
    pub optimization: String,
    pub cpu: String,
    pub llvm_report: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GccGenerator {
    pub name: String,
    pub version: String,
    pub target: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GccRemark {
    pub kind: String,
    pub pass: String,
    pub message: String,
    pub function: String,
    pub file: String,
    pub line: u64,
    pub column: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GccObservationOutcome {
    pub classification: String,
    pub selected_remarks: Vec<GccRemark>,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub output_truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GccObservationToolchain {
    pub gcc: OptimizationTool,
    pub gzip: OptimizationTool,
    pub optimization: String,
    pub cpu: String,
    pub normalized_flags: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GccComparison {
    pub llvm_analysis_id: String,
    pub llvm_semantic_digest: String,
    pub llvm_classification: String,
    pub relation: String,
    pub explanation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GccObservationReplay {
    pub function: String,
    pub line: u64,
    pub llvm_report: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GccObservationReplayResult {
    pub original_analysis_id: String,
    pub replay_analysis_id: String,
    pub semantic_digest: String,
    pub matched: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GccObservationReport {
    pub schema_version: String,
    pub analysis_id: String,
    pub query_kind: String,
    pub adapter: String,
    pub evidence_strength: String,
    pub repository: String,
    pub source: String,
    pub source_digest: String,
    pub function: String,
    pub line: u64,
    pub generator: GccGenerator,
    pub toolchain: GccObservationToolchain,
    pub outcome: GccObservationOutcome,
    pub comparison: Option<GccComparison>,
    pub replay: GccObservationReplay,
    pub semantic_digest: String,
    pub artifacts: Vec<ArtifactReference>,
    pub artifact_path: String,
    pub caveats: Vec<String>,
}

/// Observes one GCC optimization record and optionally compares its classification
/// with a retained Clang/LLVM report for the same source function and line.
///
/// # Errors
///
/// Returns `OptimizationError` when source/tool validation, compilation,
/// structured-record parsing, comparison integrity, or artifact retention fails.
#[allow(clippy::too_many_lines)]
pub fn observe_gcc_optimization(
    request: &GccObservationRequest,
) -> Result<GccObservationReport, OptimizationError> {
    validate_request(request)?;
    let repository = request.repository.canonicalize()?;
    let source = request.source.canonicalize()?;
    if !source.starts_with(&repository) {
        return Err(OptimizationError::InvalidInput(
            "GCC source escapes the repository".to_owned(),
        ));
    }
    let source_bytes = fs::read(&source)?;
    let source_digest = digest(&source_bytes);
    let analysis_id = observation_id(&source_digest, &request.function, request.line);
    let artifact_parent = repository.join(".whyvec/analyses");
    let artifact_root = artifact_parent.join(&analysis_id);
    fs::create_dir_all(&artifact_parent)?;
    fs::create_dir(&artifact_root)?;
    let store = ArtifactStore::new(&artifact_root);
    let temporary = std::env::temp_dir().join(format!("whyvec-gcc-{analysis_id}"));
    fs::create_dir(&temporary)?;
    let _cleanup = Cleanup(temporary.clone());

    let gcc = capture_tool(&request.gcc, &repository)?;
    let gzip = capture_tool(&request.gzip, &repository)?;
    let normalized_flags = vec![
        format!("-{}", request.optimization),
        format!("-march={}", request.cpu),
        "-g1".to_owned(),
        "-fsave-optimization-record".to_owned(),
        "-c".to_owned(),
    ];
    let toolchain = GccObservationToolchain {
        gcc: gcc.clone(),
        gzip: gzip.clone(),
        optimization: request.optimization.clone(),
        cpu: request.cpu.clone(),
        normalized_flags: normalized_flags.clone(),
    };
    let output_path = temporary.join("unit.o");
    let mut arguments = normalized_flags
        .iter()
        .map(OsString::from)
        .collect::<Vec<_>>();
    arguments.push(source.as_os_str().to_os_string());
    arguments.push(OsString::from("-o"));
    arguments.push(output_path.as_os_str().to_os_string());
    let compilation = execute(Path::new(&gcc.invocation_path), arguments, &temporary)?;
    if compilation.exit_code != Some(0)
        || compilation.timed_out
        || compilation.stdout_truncated
        || compilation.stderr_truncated
    {
        return Err(OptimizationError::ToolFailure(format!(
            "GCC observation compilation failed with {:?}: {}",
            compilation.exit_code,
            String::from_utf8_lossy(&compilation.stderr).trim()
        )));
    }
    let record = find_record(&temporary)?;
    let decompressed = require_success(
        execute(
            Path::new(&gzip.invocation_path),
            [OsString::from("-cd"), record.as_os_str().to_os_string()],
            &temporary,
        )?,
        "decompress GCC optimization record",
    )?;
    let document: Value = serde_json::from_slice(&decompressed.stdout)?;
    let (generator, remarks) =
        parse_record(&document, &repository, &request.function, request.line)?;
    let classification = classify(&remarks).to_owned();
    let comparison = request
        .llvm_report
        .as_deref()
        .map(|path| {
            compare_llvm(
                path,
                &source,
                &request.function,
                request.line,
                &classification,
            )
        })
        .transpose()?;

    let mut artifacts = vec![
        store.retain("inputs/source", &source_bytes, source_media_type(&source))?,
        store.retain(
            "gcc/optimization-record.json.gz",
            &fs::read(&record)?,
            "application/gzip",
        )?,
        store.retain(
            "gcc/optimization-record.json",
            &decompressed.stdout,
            "application/json",
        )?,
        store.retain(
            "gcc/stdout.bin",
            &compilation.stdout,
            "application/octet-stream",
        )?,
        store.retain(
            "gcc/stderr.bin",
            &compilation.stderr,
            "application/octet-stream",
        )?,
    ];
    artifacts.sort();
    let artifact_path = artifact_root.join("report.json");
    let mut report = GccObservationReport {
        schema_version: "2.0.0-dev".to_owned(),
        analysis_id,
        query_kind: "gcc_optimization_observation".to_owned(),
        adapter: "gcc".to_owned(),
        evidence_strength: "observed".to_owned(),
        repository: repository.to_string_lossy().into_owned(),
        source: source.to_string_lossy().into_owned(),
        source_digest,
        function: request.function.clone(),
        line: request.line,
        generator,
        toolchain,
        outcome: GccObservationOutcome {
            classification,
            selected_remarks: remarks,
            exit_code: compilation.exit_code,
            timed_out: compilation.timed_out,
            output_truncated: compilation.stdout_truncated || compilation.stderr_truncated,
        },
        comparison,
        replay: GccObservationReplay {
            function: request.function.clone(),
            line: request.line,
            llvm_report: request
                .llvm_report
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
        },
        semantic_digest: String::new(),
        artifacts,
        artifact_path: artifact_path.to_string_lossy().into_owned(),
        caveats: vec![
            "GCC optimization records are observed compiler evidence; they do not inherit LLVM assumption or source-contract semantics.".to_owned(),
            "Function and source-line matching does not provide LLVM structural loop identity; ambiguous source mappings require independent review.".to_owned(),
        ],
    };
    report.semantic_digest = report_digest(&report)?;
    store.write_new("report.json", &pretty_json(&report)?)?;
    store.finalize_read_only()?;
    Ok(report)
}

/// Re-executes a retained GCC observation and compares its semantic projection.
///
/// # Errors
///
/// Returns `OptimizationError` on artifact, source, toolchain, or outcome drift.
pub fn replay_gcc_observation(
    report_path: &Path,
) -> Result<GccObservationReplayResult, OptimizationError> {
    let original: GccObservationReport = serde_json::from_slice(&fs::read(report_path)?)?;
    let root = report_path.parent().ok_or_else(|| {
        OptimizationError::InvalidInput("GCC report path has no parent".to_owned())
    })?;
    ArtifactStore::new(root).verify(&original.artifacts)?;
    if report_digest(&original)? != original.semantic_digest {
        return Err(OptimizationError::ToolFailure(
            "GCC report semantic digest does not match its contents".to_owned(),
        ));
    }
    let source_digest = digest(&fs::read(&original.source)?);
    if source_digest != original.source_digest {
        return Err(OptimizationError::ReplayInputChanged {
            expected: original.source_digest,
            observed: source_digest,
        });
    }
    let replayed = observe_gcc_optimization(&GccObservationRequest {
        repository: PathBuf::from(&original.repository),
        source: PathBuf::from(&original.source),
        function: original.replay.function.clone(),
        line: original.replay.line,
        gcc: PathBuf::from(&original.toolchain.gcc.invocation_path),
        gzip: PathBuf::from(&original.toolchain.gzip.invocation_path),
        optimization: original.toolchain.optimization.clone(),
        cpu: original.toolchain.cpu.clone(),
        llvm_report: original.replay.llvm_report.as_ref().map(PathBuf::from),
    })?;
    if replayed.toolchain != original.toolchain {
        return Err(OptimizationError::ReplayToolchainChanged);
    }
    if replayed.semantic_digest != original.semantic_digest {
        return Err(OptimizationError::ReplayChanged {
            expected: original.semantic_digest,
            observed: replayed.semantic_digest,
        });
    }
    Ok(GccObservationReplayResult {
        original_analysis_id: original.analysis_id,
        replay_analysis_id: replayed.analysis_id,
        semantic_digest: replayed.semantic_digest,
        matched: true,
    })
}

fn validate_request(request: &GccObservationRequest) -> Result<(), OptimizationError> {
    if request.function.is_empty() || request.line == 0 {
        return Err(OptimizationError::InvalidInput(
            "GCC function and positive source line are required".to_owned(),
        ));
    }
    if request.optimization != "O3" {
        return Err(OptimizationError::InvalidInput(
            "GCC observation currently requires O3".to_owned(),
        ));
    }
    if request.cpu.is_empty() {
        return Err(OptimizationError::InvalidInput(
            "GCC CPU is required".to_owned(),
        ));
    }
    Ok(())
}

fn find_record(directory: &Path) -> Result<PathBuf, OptimizationError> {
    let mut records = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.to_string_lossy().ends_with(".opt-record.json.gz"))
        .collect::<Vec<_>>();
    records.sort();
    match records.as_slice() {
        [record] => Ok(record.clone()),
        _ => Err(OptimizationError::ToolFailure(format!(
            "expected one GCC optimization record, found {}",
            records.len()
        ))),
    }
}

fn parse_record(
    document: &Value,
    repository: &Path,
    function: &str,
    line: u64,
) -> Result<(GccGenerator, Vec<GccRemark>), OptimizationError> {
    let root = document.as_array().ok_or_else(|| {
        OptimizationError::InvalidInput("GCC optimization record root is not an array".to_owned())
    })?;
    let generator = root
        .first()
        .and_then(|item| item.get("generator"))
        .ok_or_else(|| OptimizationError::InvalidInput("GCC generator is absent".to_owned()))?;
    let generator = GccGenerator {
        name: string_field(generator, "name")?,
        version: string_field(generator, "version")?,
        target: string_field(generator, "target")?,
    };
    let mut passes = BTreeMap::new();
    collect_passes(root.get(1), &mut passes);
    let mut values = Vec::new();
    collect_remarks(root.get(2), &mut values);
    let mut remarks = values
        .into_iter()
        .filter_map(|value| normalize_remark(value, &passes, repository, function, line))
        .collect::<Vec<_>>();
    remarks.sort_by(|left, right| {
        (
            &left.file,
            left.line,
            left.column,
            &left.kind,
            &left.pass,
            &left.message,
        )
            .cmp(&(
                &right.file,
                right.line,
                right.column,
                &right.kind,
                &right.pass,
                &right.message,
            ))
    });
    remarks.dedup();
    Ok((generator, remarks))
}

fn collect_passes(value: Option<&Value>, passes: &mut BTreeMap<String, String>) {
    match value {
        Some(Value::Array(items)) => {
            for item in items {
                collect_passes(Some(item), passes);
            }
        }
        Some(Value::Object(object)) => {
            if let (Some(id), Some(name)) = (
                object.get("id").and_then(Value::as_str),
                object.get("name").and_then(Value::as_str),
            ) {
                passes.insert(id.to_owned(), name.to_owned());
            }
            collect_passes(object.get("children"), passes);
        }
        _ => {}
    }
}

fn collect_remarks<'a>(value: Option<&'a Value>, remarks: &mut Vec<&'a Value>) {
    match value {
        Some(Value::Array(items)) => {
            for item in items {
                collect_remarks(Some(item), remarks);
            }
        }
        Some(Value::Object(object)) => {
            remarks.push(value.expect("matched value is present"));
            collect_remarks(object.get("children"), remarks);
        }
        _ => {}
    }
}

fn normalize_remark(
    value: &Value,
    passes: &BTreeMap<String, String>,
    repository: &Path,
    function: &str,
    selected_line: u64,
) -> Option<GccRemark> {
    if value.get("function")?.as_str()? != function {
        return None;
    }
    let location = value.get("location")?;
    let line = location.get("line")?.as_u64()?;
    if line != selected_line {
        return None;
    }
    let raw_file = location.get("file")?.as_str()?;
    let file = Path::new(raw_file)
        .strip_prefix(repository)
        .unwrap_or_else(|_| Path::new(raw_file))
        .to_string_lossy()
        .replace('\\', "/");
    let pass_id = value
        .get("pass")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Some(GccRemark {
        kind: value.get("kind")?.as_str()?.to_owned(),
        pass: passes
            .get(pass_id)
            .cloned()
            .unwrap_or_else(|| "unknown".to_owned()),
        message: render_message(value.get("message")?),
        function: function.to_owned(),
        file,
        line,
        column: location.get("column").and_then(Value::as_u64).unwrap_or(0),
    })
}

fn render_message(value: &Value) -> String {
    let mut fragments = Vec::new();
    collect_message_fragments(value, &mut fragments);
    fragments
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_message_fragments(value: &Value, fragments: &mut Vec<String>) {
    match value {
        Value::String(text) => fragments.push(text.clone()),
        Value::Array(items) => {
            for item in items {
                collect_message_fragments(item, fragments);
            }
        }
        Value::Object(object) => {
            for (key, child) in object {
                if matches!(key.as_str(), "expr" | "stmt" | "symtab_node" | "type") {
                    collect_message_fragments(child, fragments);
                }
            }
        }
        _ => {}
    }
}

fn classify(remarks: &[GccRemark]) -> &'static str {
    if remarks.iter().any(|remark| {
        remark.kind == "success"
            && (remark.message.contains("loop vectorized")
                || remark.message.contains("loop versioned for vectorization"))
    }) {
        "vectorized"
    } else if remarks.iter().any(|remark| {
        remark.kind == "failure"
            && (remark.message.contains("not vectorized")
                || remark.message.contains("couldn't vectorize"))
    }) {
        "missed"
    } else {
        "not_observed"
    }
}

fn compare_llvm(
    path: &Path,
    source: &Path,
    function: &str,
    line: u64,
    gcc_classification: &str,
) -> Result<GccComparison, OptimizationError> {
    let llvm: OptimizationReport = serde_json::from_slice(&fs::read(path)?)?;
    let root = path.parent().ok_or_else(|| {
        OptimizationError::InvalidInput("LLVM comparison report has no parent".to_owned())
    })?;
    ArtifactStore::new(root).verify(&llvm.artifacts)?;
    if semantic_digest(&llvm)? != llvm.semantic_digest {
        return Err(OptimizationError::ToolFailure(
            "LLVM comparison report semantic digest is invalid".to_owned(),
        ));
    }
    let comparable = Path::new(&llvm.source).canonicalize()? == source
        && llvm
            .subject
            .as_ref()
            .is_some_and(|subject| subject.function == function && subject.line == line);
    let llvm_classification = llvm.monolithic_baseline.classification.clone();
    let relation = if !comparable {
        "not_comparable"
    } else if llvm_classification == gcc_classification {
        "agrees"
    } else {
        "diverges"
    };
    let explanation = match relation {
        "agrees" => {
            "GCC and Clang observed the same classification for the selected source subject."
        }
        "diverges" => {
            "GCC and Clang observed different classifications; neither compiler replaces the other's evidence."
        }
        _ => "The retained LLVM report does not identify the same canonical source subject.",
    };
    Ok(GccComparison {
        llvm_analysis_id: llvm.analysis_id,
        llvm_semantic_digest: llvm.semantic_digest,
        llvm_classification,
        relation: relation.to_owned(),
        explanation: explanation.to_owned(),
    })
}

fn report_digest(report: &GccObservationReport) -> Result<String, OptimizationError> {
    let mut value = serde_json::to_value(report)?;
    strip_non_semantic(&mut value);
    Ok(digest(&serde_json::to_vec(&value)?))
}

fn strip_non_semantic(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for field in [
                "analysis_id",
                "artifact_path",
                "artifacts",
                "repository",
                "source",
                "llvm_report",
                "semantic_digest",
            ] {
                object.remove(field);
            }
            for child in object.values_mut() {
                strip_non_semantic(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                strip_non_semantic(item);
            }
        }
        _ => {}
    }
}

fn string_field(value: &Value, field: &str) -> Result<String, OptimizationError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            OptimizationError::InvalidInput(format!("GCC generator field {field} is absent"))
        })
}

fn source_media_type(source: &Path) -> &'static str {
    match source.extension().and_then(|value| value.to_str()) {
        Some("cc" | "cpp" | "cxx") => "text/x-c++",
        _ => "text/x-c",
    }
}

fn observation_id(source_digest: &str, function: &str, line: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let material = format!(
        "{source_digest}:{function}:{line}:{now}:{}",
        std::process::id()
    );
    format!("wv_{}", &digest(material.as_bytes())[..24])
}

struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_structured_vectorization_outcomes() {
        let remark = |kind: &str, message: &str| GccRemark {
            kind: kind.to_owned(),
            pass: "vect".to_owned(),
            message: message.to_owned(),
            function: "kernel".to_owned(),
            file: "kernel.c".to_owned(),
            line: 3,
            column: 3,
        };
        assert_eq!(
            classify(&[remark("success", "loop vectorized")]),
            "vectorized"
        );
        assert_eq!(
            classify(&[remark("failure", "couldn't vectorize loop")]),
            "missed"
        );
        assert_eq!(
            classify(&[remark("note", "analysis started")]),
            "not_observed"
        );
    }

    #[test]
    fn pass_ids_are_replaced_by_stable_names() {
        let record = serde_json::json!([
            {"format":"1","generator":{"name":"GNU C","version":"15.2.0","target":"x86_64-linux-gnu"}},
            [{"id":"0x123","name":"vect","children":[]}],
            [{"kind":"failure","pass":"0x123","message":["not vectorized: unsafe dependence"],"function":"kernel","location":{"file":"/repo/kernel.c","line":3,"column":2}}]
        ]);
        let (_, remarks) =
            parse_record(&record, Path::new("/repo"), "kernel", 3).expect("record parses");
        assert_eq!(remarks[0].pass, "vect");
        assert_eq!(remarks[0].file, "kernel.c");
        assert_eq!(classify(&remarks), "missed");
    }
}
