//! Typed source access summaries for supported optimization obligations.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use whyvec_experiment::{
    ArtifactError, ArtifactReference, ArtifactStore, ProcessError, process_request, run_process,
};
use whyvec_opt::{
    OptimizationError, OptimizationReport, OptimizationTool, load_verified_optimization_report,
};

const TIMEOUT: Duration = Duration::from_mins(1);
const OUTPUT_LIMIT: usize = 64 * 1024 * 1024;

type AnalysisResult = (
    Option<DerivedObligation>,
    Option<ObligationDecline>,
    Vec<ArtifactReference>,
);

#[derive(Clone, Debug)]
pub struct ObligationRequest {
    pub optimization_report: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceEntity {
    pub name: String,
    pub source_type: String,
    pub byte_width: u64,
    pub signed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InductionDomain {
    pub variable: String,
    pub source_type: String,
    pub lower_bound: i64,
    pub upper_bound_parameter: String,
    pub comparison: String,
    pub step: u64,
    pub zero_trip_when_upper_at_most_lower: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WriteRegion {
    pub base_parameter: String,
    pub element_type: String,
    pub element_bytes: u64,
    pub index_expression: String,
    pub conservative_first_index: String,
    pub conservative_last_index: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccessSummary {
    pub model: String,
    pub function: String,
    pub loop_line: u64,
    pub bound_object: SourceEntity,
    pub induction: InductionDomain,
    pub writes: Vec<WriteRegion>,
    pub calls_in_loop: usize,
    pub volatile_accesses: usize,
    pub atomic_accesses: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeGuardPlan {
    pub target_policy: String,
    pub capture: String,
    pub checked_operations: Vec<String>,
    pub condition: String,
    pub fast_path_requirement: String,
    pub fallback_requirement: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DerivedObligation {
    pub evidence_strength: String,
    pub family: String,
    pub candidate_assumption: String,
    pub predicate: String,
    pub access_summary: AccessSummary,
    pub runtime_guard: RuntimeGuardPlan,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObligationDecline {
    pub code: String,
    pub explanation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObligationReplay {
    pub optimization_report: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObligationReport {
    pub schema_version: String,
    pub analysis_id: String,
    pub query_kind: String,
    pub adapter: String,
    pub repository: String,
    pub source: String,
    pub source_digest: String,
    pub optimization_analysis_id: String,
    pub optimization_semantic_digest: String,
    pub clang: OptimizationTool,
    pub obligation: Option<DerivedObligation>,
    pub decline: Option<ObligationDecline>,
    pub replay: ObligationReplay,
    pub semantic_digest: String,
    pub artifacts: Vec<ArtifactReference>,
    pub artifact_path: String,
    pub caveats: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObligationReplayResult {
    pub original_analysis_id: String,
    pub replay_analysis_id: String,
    pub semantic_digest: String,
    pub matched: bool,
}

#[derive(Debug)]
pub enum ObligationError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Artifact(ArtifactError),
    Process(ProcessError),
    Optimization(OptimizationError),
    InvalidInput(String),
    ToolFailure(String),
    ReplayChanged { expected: String, observed: String },
}

impl std::fmt::Display for ObligationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::Json(error) => error.fmt(formatter),
            Self::Artifact(error) => error.fmt(formatter),
            Self::Process(error) => error.fmt(formatter),
            Self::Optimization(error) => error.fmt(formatter),
            Self::InvalidInput(detail) => write!(formatter, "invalid obligation query: {detail}"),
            Self::ToolFailure(detail) => write!(formatter, "obligation tool failed: {detail}"),
            Self::ReplayChanged { expected, observed } => write!(
                formatter,
                "obligation replay semantic digest changed (expected {expected}, observed {observed})"
            ),
        }
    }
}

impl std::error::Error for ObligationError {}

macro_rules! from_error {
    ($source:ty, $variant:ident) => {
        impl From<$source> for ObligationError {
            fn from(value: $source) -> Self {
                Self::$variant(value)
            }
        }
    };
}

from_error!(std::io::Error, Io);
from_error!(serde_json::Error, Json);
from_error!(ArtifactError, Artifact);
from_error!(ProcessError, Process);
from_error!(OptimizationError, Optimization);

/// Derives a source-level candidate obligation through the declared affine
/// pointer-bound access model, or retains a typed decline.
///
/// # Errors
///
/// Returns `ObligationError` for invalid retained evidence, tool execution, or
/// artifact persistence failures. Unsupported source semantics are reports,
/// not execution errors.
#[allow(clippy::too_many_lines)]
pub fn derive_obligation(request: &ObligationRequest) -> Result<ObligationReport, ObligationError> {
    let optimization = load_verified_optimization_report(&request.optimization_report)?;
    let source = PathBuf::from(&optimization.source).canonicalize()?;
    let repository = PathBuf::from(&optimization.repository).canonicalize()?;
    let source_bytes = fs::read(&source)?;
    if digest(&source_bytes) != optimization.source_digest {
        return Err(ObligationError::InvalidInput(
            "optimization source digest changed".to_owned(),
        ));
    }
    let analysis_id = analysis_id(&optimization);
    let root = repository.join(".whyvec/analyses").join(&analysis_id);
    fs::create_dir(&root)?;
    let store = ArtifactStore::new(&root);
    let artifact_path = root.join("report.json");
    let mut artifacts =
        vec![store.retain("inputs/source", &source_bytes, source_media_type(&source))?];

    let (obligation, decline, ast_artifacts) =
        analyze(&optimization, &source, &repository, &store)?;
    artifacts.extend(ast_artifacts);
    artifacts.sort();
    artifacts.dedup();
    let mut report = ObligationReport {
        schema_version: "2.0.0-dev".to_owned(),
        analysis_id,
        query_kind: "obligation_derivation".to_owned(),
        adapter: "clang_ast_affine_bound_v1".to_owned(),
        repository: repository.to_string_lossy().into_owned(),
        source: source.to_string_lossy().into_owned(),
        source_digest: optimization.source_digest.clone(),
        optimization_analysis_id: optimization.analysis_id,
        optimization_semantic_digest: optimization.semantic_digest,
        clang: optimization.toolchain.clang,
        obligation,
        decline,
        replay: ObligationReplay {
            optimization_report: request.optimization_report.to_string_lossy().into_owned(),
        },
        semantic_digest: String::new(),
        artifacts,
        artifact_path: artifact_path.to_string_lossy().into_owned(),
        caveats: vec![
            "The LLVM parameter assumption and this candidate source obligation remain distinct evidence.".to_owned(),
            "The affine access model covers the selected source loop only; repository callers and concurrency contracts are not inferred.".to_owned(),
        ],
    };
    report.semantic_digest = report_digest(&report)?;
    store.write_new("report.json", &pretty_json(&report)?)?;
    store.finalize_read_only()?;
    Ok(report)
}

/// Re-executes a retained obligation derivation and verifies its semantic result.
///
/// # Errors
///
/// Returns `ObligationError` on artifact, optimization evidence, tool, source,
/// or semantic drift.
pub fn replay_obligation(report_path: &Path) -> Result<ObligationReplayResult, ObligationError> {
    let original: ObligationReport = serde_json::from_slice(&fs::read(report_path)?)?;
    let root = report_path.parent().ok_or_else(|| {
        ObligationError::InvalidInput("obligation report path has no parent".to_owned())
    })?;
    ArtifactStore::new(root).verify(&original.artifacts)?;
    if report_digest(&original)? != original.semantic_digest {
        return Err(ObligationError::InvalidInput(
            "obligation report semantic digest is invalid".to_owned(),
        ));
    }
    let replayed = derive_obligation(&ObligationRequest {
        optimization_report: PathBuf::from(&original.replay.optimization_report),
    })?;
    if replayed.clang != original.clang {
        return Err(ObligationError::InvalidInput(
            "obligation Clang fingerprint changed".to_owned(),
        ));
    }
    if replayed.semantic_digest != original.semantic_digest {
        return Err(ObligationError::ReplayChanged {
            expected: original.semantic_digest,
            observed: replayed.semantic_digest,
        });
    }
    Ok(ObligationReplayResult {
        original_analysis_id: original.analysis_id,
        replay_analysis_id: replayed.analysis_id,
        semantic_digest: replayed.semantic_digest,
        matched: true,
    })
}

#[allow(clippy::too_many_lines)]
fn analyze(
    optimization: &OptimizationReport,
    source: &Path,
    repository: &Path,
    store: &ArtifactStore,
) -> Result<AnalysisResult, ObligationError> {
    if !matches!(
        source.extension().and_then(|value| value.to_str()),
        Some("c")
    ) {
        return Ok(declined(
            "obligation.language_unsupported",
            "the first source access model supports C translation units only",
        ));
    }
    let Some(subject) = optimization.subject.as_ref() else {
        return Ok(declined(
            "obligation.subject_unavailable",
            "the optimization report has no uniquely selected loop subject",
        ));
    };
    let clang = Path::new(&optimization.toolchain.clang.invocation_path);
    let mut request = process_request(
        clang,
        [
            OsString::from("-Xclang"),
            OsString::from("-ast-dump=json"),
            OsString::from("-fsyntax-only"),
            source.as_os_str().to_os_string(),
        ],
        repository,
    );
    request.timeout = TIMEOUT;
    request.output_limit = OUTPUT_LIMIT;
    let output = run_process(&request)?;
    if output.exit_code != Some(0)
        || output.timed_out
        || output.stdout_truncated
        || output.stderr_truncated
    {
        return Err(ObligationError::ToolFailure(format!(
            "Clang AST extraction failed with {:?}: {}",
            output.exit_code,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let artifacts = vec![
        store.retain("ast/clang.json", &output.stdout, "application/json")?,
        store.retain("ast/stderr.txt", &output.stderr, "text/plain")?,
    ];
    let ast: Value = serde_json::from_slice(&output.stdout)?;
    let loops = find_loops(&ast, &subject.function, subject.line);
    let [selected] = loops.as_slice() else {
        let (code, explanation) = if loops.is_empty() {
            (
                "obligation.loop_absent",
                "Clang AST did not contain the selected source loop",
            )
        } else {
            (
                "obligation.loop_ambiguous",
                "multiple Clang AST loops match the selected function and line",
            )
        };
        let (obligation, decline, _) = declined(code, explanation);
        return Ok((obligation, decline, artifacts));
    };
    let analysis = analyze_loop(selected, &subject.function, subject.line);
    match analysis {
        Ok(summary) => {
            if optimization.pipeline_fidelity != "equivalent_confirmed" {
                let (obligation, decline, _) = declined(
                    "obligation.pipeline_fidelity",
                    "source obligation evaluation requires equivalent-confirmed pipeline fidelity",
                );
                return Ok((obligation, decline, artifacts));
            }
            let Some(finding) = optimization.finding.as_ref() else {
                let (obligation, decline, _) = declined(
                    "obligation.no_sufficient_assumption",
                    "the optimization report contains no tested sufficient assumption",
                );
                return Ok((obligation, decline, artifacts));
            };
            let expected = format!("parameter.{}.noalias", summary.bound_object.name);
            if finding.sufficient_assumptions != [expected.as_str()] {
                let (obligation, decline, _) = declined(
                    "obligation.assumption_mismatch",
                    "the tested sufficient assumption is not the modeled bound parameter",
                );
                return Ok((obligation, decline, artifacts));
            }
            let writes = summary
                .writes
                .iter()
                .map(|write| write.base_parameter.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let predicate = format!(
                "bytes({}, sizeof(*{})) is disjoint from every modified byte in [{}] over the initial iteration domain",
                summary.bound_object.name, summary.bound_object.name, writes
            );
            let guard = guard_plan(&summary);
            Ok((
                Some(DerivedObligation {
                    evidence_strength: "derived_obligation".to_owned(),
                    family: "bound_object_disjoint_from_modified_region".to_owned(),
                    candidate_assumption: expected,
                    predicate,
                    access_summary: summary,
                    runtime_guard: guard,
                    limitations: vec![
                        "No claim is made that callers already satisfy this condition.".to_owned(),
                        "The runtime plan requires the recorded flat uintptr_t target policy and checked arithmetic.".to_owned(),
                    ],
                }),
                None,
                artifacts,
            ))
        }
        Err(decline) => Ok((None, Some(decline), artifacts)),
    }
}

fn declined(
    code: &str,
    explanation: &str,
) -> (
    Option<DerivedObligation>,
    Option<ObligationDecline>,
    Vec<ArtifactReference>,
) {
    (
        None,
        Some(ObligationDecline {
            code: code.to_owned(),
            explanation: explanation.to_owned(),
        }),
        Vec::new(),
    )
}

fn find_loops<'a>(root: &'a Value, function: &str, line: u64) -> Vec<&'a Value> {
    let mut functions = Vec::new();
    walk(root, &mut |node| {
        if node.get("kind").and_then(Value::as_str) == Some("FunctionDecl")
            && node.get("name").and_then(Value::as_str) == Some(function)
        {
            functions.push(node);
        }
    });
    let mut loops = Vec::new();
    for function in functions {
        walk(function, &mut |node| {
            if node.get("kind").and_then(Value::as_str) == Some("ForStmt")
                && node.pointer("/range/begin/line").and_then(Value::as_u64) == Some(line)
            {
                loops.push(node);
            }
        });
    }
    loops
}

fn walk<'a>(value: &'a Value, visit: &mut impl FnMut(&'a Value)) {
    visit(value);
    if let Some(children) = value.get("inner").and_then(Value::as_array) {
        for child in children {
            walk(child, visit);
        }
    }
}

fn analyze_loop(
    loop_node: &Value,
    function: &str,
    line: u64,
) -> Result<AccessSummary, ObligationDecline> {
    let mut volatile = 0;
    let mut atomic = 0;
    let mut calls = 0;
    walk(loop_node, &mut |node| {
        let kind = node.get("kind").and_then(Value::as_str).unwrap_or_default();
        if kind.contains("CallExpr") || kind == "CallExpr" {
            calls += 1;
        }
        let source_type = qual_type(node).unwrap_or_default();
        if source_type.contains("volatile") {
            volatile += 1;
        }
        if source_type.contains("_Atomic") || kind.contains("Atomic") {
            atomic += 1;
        }
    });
    if volatile > 0 {
        return Err(ObligationDecline {
            code: "obligation.volatile_bound".to_owned(),
            explanation: "the selected loop contains volatile access semantics".to_owned(),
        });
    }
    if atomic > 0 {
        return Err(ObligationDecline {
            code: "obligation.atomic_access".to_owned(),
            explanation: "the selected loop contains atomic access semantics".to_owned(),
        });
    }
    if calls > 0 {
        return Err(ObligationDecline {
            code: "obligation.call_in_loop".to_owned(),
            explanation: "the selected loop contains a call without an access summary".to_owned(),
        });
    }
    let children = loop_node
        .get("inner")
        .and_then(Value::as_array)
        .ok_or_else(|| decline("obligation.loop_shape", "ForStmt children are absent"))?;
    let induction = parse_induction(children)?;
    let (bound_name, bound_pointer_type) = parse_condition(children, &induction.variable)?;
    let (bound_type, bound_bytes, signed) =
        pointee_layout(&bound_pointer_type).ok_or_else(|| {
            decline(
                "obligation.bound_layout",
                "the pointer-loaded bound type has no supported fixed layout",
            )
        })?;
    let writes = parse_writes(children, &induction.variable, induction.lower_bound)?;
    if writes.is_empty() {
        return Err(decline(
            "obligation.write_absent",
            "the selected loop has no supported indexed write",
        ));
    }
    Ok(AccessSummary {
        model: "clang_ast_affine_bound_v1".to_owned(),
        function: function.to_owned(),
        loop_line: line,
        bound_object: SourceEntity {
            name: bound_name.clone(),
            source_type: bound_type,
            byte_width: bound_bytes,
            signed,
        },
        induction: InductionDomain {
            variable: induction.variable,
            source_type: induction.source_type,
            lower_bound: induction.lower_bound,
            upper_bound_parameter: bound_name,
            comparison: induction.comparison,
            step: induction.step,
            zero_trip_when_upper_at_most_lower: true,
        },
        writes,
        calls_in_loop: calls,
        volatile_accesses: volatile,
        atomic_accesses: atomic,
    })
}

struct ParsedInduction {
    variable: String,
    source_type: String,
    lower_bound: i64,
    comparison: String,
    step: u64,
}

fn parse_induction(children: &[Value]) -> Result<ParsedInduction, ObligationDecline> {
    let declaration = children
        .iter()
        .find(|node| node.get("kind").and_then(Value::as_str) == Some("DeclStmt"))
        .and_then(|node| node.get("inner"))
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .ok_or_else(|| {
            decline(
                "obligation.induction_init",
                "loop induction declaration is unsupported",
            )
        })?;
    let variable = declaration
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            decline(
                "obligation.induction_init",
                "induction variable name is absent",
            )
        })?
        .to_owned();
    let source_type = qual_type(declaration)
        .ok_or_else(|| decline("obligation.induction_type", "induction type is absent"))?
        .to_owned();
    let lower_bound = first_integer(declaration).ok_or_else(|| {
        decline(
            "obligation.induction_init",
            "induction lower bound is not an integer constant",
        )
    })?;
    let increment = children
        .iter()
        .find(|node| {
            matches!(
                node.get("kind").and_then(Value::as_str),
                Some("UnaryOperator" | "CompoundAssignOperator")
            ) && contains_reference(node, &variable)
        })
        .ok_or_else(|| decline("obligation.induction_step", "induction increment is absent"))?;
    let step = match (
        increment.get("kind").and_then(Value::as_str),
        increment.get("opcode").and_then(Value::as_str),
    ) {
        (Some("UnaryOperator"), Some("++")) => 1,
        (Some("CompoundAssignOperator"), Some("+=")) => {
            u64::try_from(first_integer(increment).ok_or_else(|| {
                decline(
                    "obligation.induction_step",
                    "induction step is not constant",
                )
            })?)
            .map_err(|_| {
                decline(
                    "obligation.induction_step",
                    "induction step is not positive",
                )
            })?
        }
        _ => {
            return Err(decline(
                "obligation.induction_step",
                "only constant positive increments are supported",
            ));
        }
    };
    if step == 0 {
        return Err(decline(
            "obligation.induction_step",
            "zero induction step is unsupported",
        ));
    }
    Ok(ParsedInduction {
        variable,
        source_type,
        lower_bound,
        comparison: "less_than".to_owned(),
        step,
    })
}

fn parse_condition(
    children: &[Value],
    induction: &str,
) -> Result<(String, String), ObligationDecline> {
    let condition = children
        .iter()
        .find(|node| {
            node.get("kind").and_then(Value::as_str) == Some("BinaryOperator")
                && node.get("opcode").and_then(Value::as_str) == Some("<")
                && contains_reference(node, induction)
        })
        .ok_or_else(|| {
            decline(
                "obligation.loop_condition",
                "only induction < pointer-loaded-bound conditions are supported",
            )
        })?;
    let mut dereferences = Vec::new();
    walk(condition, &mut |node| {
        if node.get("kind").and_then(Value::as_str) == Some("UnaryOperator")
            && node.get("opcode").and_then(Value::as_str) == Some("*")
        {
            dereferences.push(node);
        }
    });
    let [dereference] = dereferences.as_slice() else {
        return Err(decline(
            "obligation.bound_load",
            "loop condition must contain one pointer-loaded bound",
        ));
    };
    parameter_reference(dereference).ok_or_else(|| {
        decline(
            "obligation.bound_load",
            "bound load is not based on one function parameter",
        )
    })
}

fn parse_writes(
    children: &[Value],
    induction: &str,
    lower_bound: i64,
) -> Result<Vec<WriteRegion>, ObligationDecline> {
    let mut assignments = Vec::new();
    for child in children {
        walk(child, &mut |node| {
            if matches!(
                node.get("kind").and_then(Value::as_str),
                Some("BinaryOperator" | "CompoundAssignOperator")
            ) && matches!(
                node.get("opcode").and_then(Value::as_str),
                Some("=" | "+=" | "-=" | "*=" | "/=")
            ) {
                assignments.push(node);
            }
        });
    }
    let mut writes = Vec::new();
    for assignment in assignments {
        let Some(lhs) = assignment
            .get("inner")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
        else {
            continue;
        };
        if lhs.get("kind").and_then(Value::as_str) != Some("ArraySubscriptExpr") {
            return Err(decline(
                "obligation.write_shape",
                "loop write is not a supported array subscript",
            ));
        }
        let operands = lhs
            .get("inner")
            .and_then(Value::as_array)
            .ok_or_else(|| decline("obligation.write_shape", "array operands are absent"))?;
        let base = operands
            .first()
            .and_then(parameter_reference)
            .ok_or_else(|| decline("obligation.write_base", "write base is not a parameter"))?;
        let index = operands
            .get(1)
            .ok_or_else(|| decline("obligation.write_index", "array write index is absent"))?;
        if !is_direct_reference(index, induction) {
            return Err(decline(
                "obligation.non_affine_index",
                "the first access model requires the induction variable as the direct write index",
            ));
        }
        let element_type = qual_type(lhs)
            .ok_or_else(|| decline("obligation.write_layout", "write element type is absent"))?;
        let element_bytes = scalar_layout(element_type).ok_or_else(|| {
            decline(
                "obligation.write_layout",
                "write element type has no supported fixed layout",
            )
        })?;
        writes.push(WriteRegion {
            base_parameter: base.0,
            element_type: element_type.to_owned(),
            element_bytes,
            index_expression: induction.to_owned(),
            conservative_first_index: lower_bound.to_string(),
            conservative_last_index: "initial(*bound) - 1 rounded by step".to_owned(),
        });
    }
    writes.sort_by(|left, right| left.base_parameter.cmp(&right.base_parameter));
    writes.dedup();
    Ok(writes)
}

fn guard_plan(summary: &AccessSummary) -> RuntimeGuardPlan {
    let bound = &summary.bound_object.name;
    let writes = summary
        .writes
        .iter()
        .map(|write| {
            format!(
                "range({}, checked_extent({}, {}, {}, {}))",
                write.base_parameter,
                summary.induction.lower_bound,
                format_args!("initial(*{bound})"),
                summary.induction.step,
                write.element_bytes
            )
        })
        .collect::<Vec<_>>()
        .join(" and ");
    RuntimeGuardPlan {
        target_policy: "flat_uintptr_x86_64".to_owned(),
        capture: format!("initial_bound = *{bound} before assuming disjointness"),
        checked_operations: vec![
            "checked iteration-count calculation".to_owned(),
            "checked element-count multiplication".to_owned(),
            "checked uintptr_t range-end addition".to_owned(),
        ],
        condition: format!(
            "zero-trip or bytes({bound}, {}) is disjoint from {writes}",
            summary.bound_object.byte_width
        ),
        fast_path_requirement:
            "the guard dominates bound caching and every optimized-path assumption".to_owned(),
        fallback_requirement: "execute the original loop with its pointer-loaded bound unchanged"
            .to_owned(),
    }
}

fn parameter_reference(value: &Value) -> Option<(String, String)> {
    let mut found = None;
    walk(value, &mut |node| {
        let Some(declaration) = node.get("referencedDecl") else {
            return;
        };
        if declaration.get("kind").and_then(Value::as_str) == Some("ParmVarDecl")
            && let (Some(name), Some(source_type)) = (
                declaration.get("name").and_then(Value::as_str),
                declaration
                    .pointer("/type/qualType")
                    .and_then(Value::as_str),
            )
        {
            found.get_or_insert((name.to_owned(), source_type.to_owned()));
        }
    });
    found
}

fn contains_reference(value: &Value, name: &str) -> bool {
    let mut found = false;
    walk(value, &mut |node| {
        if node.pointer("/referencedDecl/name").and_then(Value::as_str) == Some(name) {
            found = true;
        }
    });
    found
}

fn is_direct_reference(value: &Value, name: &str) -> bool {
    let mut names = Vec::new();
    walk(value, &mut |node| {
        if let Some(reference) = node.pointer("/referencedDecl/name").and_then(Value::as_str) {
            names.push(reference);
        }
    });
    names == [name] && !contains_kind(value, "BinaryOperator") && !contains_kind(value, "CallExpr")
}

fn contains_kind(value: &Value, kind: &str) -> bool {
    let mut found = false;
    walk(value, &mut |node| {
        if node.get("kind").and_then(Value::as_str) == Some(kind) {
            found = true;
        }
    });
    found
}

fn first_integer(value: &Value) -> Option<i64> {
    let mut found = None;
    walk(value, &mut |node| {
        if node.get("kind").and_then(Value::as_str) == Some("IntegerLiteral") {
            found = node
                .get("value")
                .and_then(Value::as_str)
                .and_then(|raw| raw.parse().ok());
        }
    });
    found
}

fn qual_type(value: &Value) -> Option<&str> {
    value.pointer("/type/qualType")?.as_str()
}

fn pointee_layout(pointer: &str) -> Option<(String, u64, bool)> {
    let element_type = pointer
        .strip_suffix('*')?
        .trim()
        .trim_start_matches("const ")
        .trim_start_matches("volatile ")
        .trim();
    scalar_layout(element_type).map(|bytes| {
        (
            element_type.to_owned(),
            bytes,
            !element_type.starts_with("unsigned"),
        )
    })
}

fn scalar_layout(source_type: &str) -> Option<u64> {
    match source_type.trim_start_matches("const ").trim() {
        "char" | "signed char" | "unsigned char" => Some(1),
        "short" | "short int" | "unsigned short" | "unsigned short int" => Some(2),
        "int" | "unsigned" | "unsigned int" | "float" => Some(4),
        "long" | "long int" | "unsigned long" | "unsigned long int" | "long long"
        | "long long int" | "unsigned long long" | "double" => Some(8),
        _ => None,
    }
}

fn decline(code: &str, explanation: &str) -> ObligationDecline {
    ObligationDecline {
        code: code.to_owned(),
        explanation: explanation.to_owned(),
    }
}

fn report_digest(report: &ObligationReport) -> Result<String, ObligationError> {
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
                "optimization_report",
                "semantic_digest",
            ] {
                object.remove(field);
            }
            for child in object.values_mut() {
                strip_non_semantic(child);
            }
        }
        Value::Array(items) => {
            for child in items {
                strip_non_semantic(child);
            }
        }
        _ => {}
    }
}

fn pretty_json(value: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to String cannot fail");
            output
        })
}

fn analysis_id(optimization: &OptimizationReport) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let material = format!(
        "{}:{}:{now}:{}",
        optimization.semantic_digest,
        optimization.analysis_id,
        std::process::id()
    );
    format!("wv_{}", &digest(material.as_bytes())[..24])
}

fn source_media_type(source: &Path) -> &'static str {
    if source.extension().and_then(|value| value.to_str()) == Some("c") {
        "text/x-c"
    } else {
        "application/octet-stream"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_layout_is_explicit_and_target_bounded() {
        assert_eq!(
            pointee_layout("const int *"),
            Some(("int".to_owned(), 4, true))
        );
        assert_eq!(
            pointee_layout("const unsigned long *"),
            Some(("unsigned long".to_owned(), 8, false))
        );
        assert_eq!(pointee_layout("const __int128 *"), None);
    }

    #[test]
    fn volatile_and_atomic_dimensions_have_distinct_declines() {
        let volatile =
            serde_json::json!({"kind":"ForStmt","type":{"qualType":"volatile int"},"inner":[]});
        assert!(matches!(
            analyze_loop(&volatile, "kernel", 3),
            Err(ObligationDecline { code, .. }) if code == "obligation.volatile_bound"
        ));
        let atomic =
            serde_json::json!({"kind":"ForStmt","type":{"qualType":"_Atomic(int)"},"inner":[]});
        assert!(matches!(
            analyze_loop(&atomic, "kernel", 3),
            Err(ObligationDecline { code, .. }) if code == "obligation.atomic_access"
        ));
    }

    #[test]
    fn checked_range_end_refuses_overflow() {
        assert_eq!(checked_range(100, 8), Some((100, 108)));
        assert_eq!(checked_range(u64::MAX - 2, 8), None);
        assert_eq!(checked_extent(4, 8), Some(32));
        assert_eq!(checked_extent(u64::MAX, 2), None);
    }

    fn checked_range(start: u64, bytes: u64) -> Option<(u64, u64)> {
        Some((start, start.checked_add(bytes)?))
    }

    fn checked_extent(elements: u64, bytes: u64) -> Option<u64> {
        elements.checked_mul(bytes)
    }
}
