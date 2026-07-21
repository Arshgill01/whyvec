//! Retained LLVM optimization-causality experiments.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use whyvec_domain::{ExperimentVerdict, SearchMinimality, UnresolvedReason};
use whyvec_experiment::{
    ArtifactError, ArtifactReference, ArtifactStore, InterventionId, ProcessError, ProcessResult,
    SearchConfigurationError, SearchLimits, SearchStopReason, process_request, run_process,
    search_sufficient_sets,
};

const TIMEOUT: Duration = Duration::from_secs(90);
const OUTPUT_LIMIT: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterCandidate {
    pub source_name: String,
    pub ir_index: usize,
}

#[derive(Clone, Debug)]
pub struct OptimizationRequest {
    pub repository: PathBuf,
    pub source: PathBuf,
    pub function: String,
    pub line: u64,
    pub candidates: Vec<ParameterCandidate>,
    pub clang: PathBuf,
    pub optimizer: PathBuf,
    pub transformer: PathBuf,
    pub identity_tool: PathBuf,
    pub optimization: String,
    pub cpu: String,
    pub max_evaluations: usize,
    pub max_cardinality: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopIdentity {
    pub function: String,
    pub line: u64,
    pub column: u64,
    pub loop_depth: u64,
    pub block_count: u64,
    pub structural_fingerprint: String,
    pub mapping_confidence: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptimizationOutcome {
    pub classification: String,
    pub vector_factor: Option<u64>,
    pub interleave_count: Option<u64>,
    pub selected_remarks: Vec<String>,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub output_truncated: bool,
    pub confirmation_runs: usize,
    pub consistent: bool,
    pub artifacts: Vec<ArtifactReference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptimizationExperiment {
    pub experiment_id: String,
    pub assumptions: Vec<String>,
    pub verdict: String,
    pub unresolved_reason: Option<String>,
    pub ir_verified: bool,
    pub delta_isolated: bool,
    pub loop_identity: LoopIdentity,
    pub outcome: OptimizationOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptimizationFinding {
    pub evidence_strength: String,
    pub sufficient_assumptions: Vec<String>,
    pub minimality: String,
    pub summary: String,
    pub caveats: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptimizationDecline {
    pub code: String,
    pub explanation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptimizationTool {
    pub invocation_path: String,
    pub resolved_path: String,
    pub binary_digest: String,
    pub version: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptimizationToolchain {
    pub clang: OptimizationTool,
    pub optimizer: OptimizationTool,
    pub transformer: OptimizationTool,
    pub identity_tool: OptimizationTool,
    pub optimization: String,
    pub cpu: String,
    pub normalized_flags: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptimizationReplay {
    pub max_evaluations: usize,
    pub max_cardinality: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptimizationReplayResult {
    pub original_analysis_id: String,
    pub replay_analysis_id: String,
    pub semantic_digest: String,
    pub matched: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptimizationReport {
    pub schema_version: String,
    pub analysis_id: String,
    pub query_kind: String,
    pub adapter: String,
    pub pipeline_fidelity: String,
    pub toolchain: OptimizationToolchain,
    pub repository: String,
    pub source: String,
    pub source_digest: String,
    pub pipeline_digest: String,
    pub subject: LoopIdentity,
    pub candidates: Vec<OptimizationCandidateReport>,
    pub monolithic_baseline: OptimizationOutcome,
    pub replay_baseline: OptimizationOutcome,
    pub experiments: Vec<OptimizationExperiment>,
    pub minimality: String,
    pub stop_reason: String,
    pub declared_subsets: usize,
    pub finding: Option<OptimizationFinding>,
    pub decline: Option<OptimizationDecline>,
    pub replay: OptimizationReplay,
    pub semantic_digest: String,
    pub artifacts: Vec<ArtifactReference>,
    pub artifact_path: String,
    pub caveats: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptimizationCandidateReport {
    pub id: String,
    pub source_name: String,
    pub ir_index: usize,
    pub attribute: String,
}

#[derive(Debug)]
pub enum OptimizationError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Artifact(ArtifactError),
    Process(ProcessError),
    Search(SearchConfigurationError),
    InvalidInput(String),
    ToolFailure(String),
    ReplayInputChanged { expected: String, observed: String },
    ReplayToolchainChanged,
    ReplayChanged { expected: String, observed: String },
}

impl std::fmt::Display for OptimizationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::Json(error) => error.fmt(formatter),
            Self::Artifact(error) => error.fmt(formatter),
            Self::Process(error) => error.fmt(formatter),
            Self::Search(error) => error.fmt(formatter),
            Self::InvalidInput(detail) => write!(formatter, "invalid optimization query: {detail}"),
            Self::ToolFailure(detail) => write!(formatter, "optimization tool failed: {detail}"),
            Self::ReplayInputChanged { expected, observed } => write!(
                formatter,
                "optimization replay source changed (expected {expected}, observed {observed})"
            ),
            Self::ReplayToolchainChanged => {
                formatter.write_str("optimization replay toolchain changed")
            }
            Self::ReplayChanged { expected, observed } => write!(
                formatter,
                "optimization replay semantic digest changed (expected {expected}, observed {observed})"
            ),
        }
    }
}

impl std::error::Error for OptimizationError {}

macro_rules! from_error {
    ($source:ty, $variant:ident) => {
        impl From<$source> for OptimizationError {
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
from_error!(SearchConfigurationError, Search);

/// Runs and retains one Clang/LLVM parameter-noalias optimization query.
///
/// # Errors
///
/// Returns `OptimizationError` when inputs, tools, artifacts, or search execution fail.
#[allow(clippy::too_many_lines)]
pub fn explain_optimization(
    request: &OptimizationRequest,
) -> Result<OptimizationReport, OptimizationError> {
    validate_request(request)?;
    let repository = request.repository.canonicalize()?;
    let source = request.source.canonicalize()?;
    if !source.starts_with(&repository) {
        return Err(OptimizationError::InvalidInput(
            "source escapes the repository".to_owned(),
        ));
    }
    let source_bytes = fs::read(&source)?;
    let source_digest = digest(&source_bytes);
    let analysis_id = analysis_id(&source_digest, request);
    let artifact_parent = repository.join(".whyvec/analyses");
    let root = artifact_parent.join(&analysis_id);
    fs::create_dir_all(&artifact_parent)?;
    fs::create_dir(&root)?;
    let store = ArtifactStore::new(&root);
    let mut artifacts = vec![store.retain("inputs/source.c", &source_bytes, "text/x-c")?];
    let toolchain = capture_toolchain(request, &repository)?;
    artifacts.push(store.retain(
        "inputs/toolchain.json",
        &pretty_json(&toolchain)?,
        "application/json",
    )?);
    let temporary = std::env::temp_dir().join(format!("whyvec-opt-{analysis_id}"));
    fs::create_dir(&temporary)?;
    let _cleanup = Cleanup(temporary.clone());

    let common = common_clang_arguments(request, &source);
    let monolithic_record = temporary.join("baseline.opt.yaml");
    let monolithic = execute(
        &request.clang,
        common.iter().cloned().chain([
            OsString::from("-Rpass=loop-vectorize"),
            OsString::from("-Rpass-missed=loop-vectorize"),
            OsString::from("-Rpass-analysis=loop-vectorize"),
            OsString::from("-fsave-optimization-record=yaml"),
            OsString::from(format!(
                "-foptimization-record-file={}",
                monolithic_record.display()
            )),
            OsString::from("-c"),
            source.as_os_str().to_os_string(),
            OsString::from("-o"),
            temporary.join("baseline.o").into_os_string(),
        ]),
        &repository,
    )?;
    let mut monolithic_baseline = retain_outcome(
        &store,
        "baseline/monolithic",
        &monolithic,
        &source,
        request.line,
    )?;
    let monolithic_record_artifact = store.retain(
        "baseline/monolithic.opt.yaml",
        &fs::read(&monolithic_record)?,
        "application/yaml",
    )?;
    monolithic_baseline
        .artifacts
        .push(monolithic_record_artifact.clone());
    artifacts.push(monolithic_record_artifact);
    artifacts.extend(monolithic_baseline.artifacts.iter().cloned());

    let preopt = temporary.join("preopt.ll");
    require_success(
        execute(
            &request.clang,
            common.iter().cloned().chain([
                OsString::from("-Xclang"),
                OsString::from("-disable-llvm-passes"),
                OsString::from("-emit-llvm"),
                OsString::from("-S"),
                source.as_os_str().to_os_string(),
                OsString::from("-o"),
                preopt.as_os_str().to_os_string(),
            ]),
            &repository,
        )?,
        "emit pre-optimization IR",
    )?;
    artifacts.push(store.retain("baseline/preopt.ll", &fs::read(&preopt)?, "text/plain")?);

    let pipeline_run = execute(
        &request.clang,
        common.iter().cloned().chain([
            OsString::from("-mllvm"),
            OsString::from("-print-pipeline-passes"),
            OsString::from("-c"),
            source.as_os_str().to_os_string(),
            OsString::from("-o"),
            temporary.join("pipeline.o").into_os_string(),
        ]),
        &repository,
    )?;
    require_success(pipeline_run.clone(), "capture Clang pipeline")?;
    let pipeline = String::from_utf8_lossy(&pipeline_run.stdout)
        .trim()
        .to_owned();
    if !pipeline.contains("loop-vectorize") {
        return Err(OptimizationError::ToolFailure(
            "captured pipeline lacks loop-vectorize".to_owned(),
        ));
    }
    artifacts.push(store.retain("baseline/pipeline.txt", pipeline.as_bytes(), "text/plain")?);
    let pipeline_digest = digest(pipeline.as_bytes());

    let subject = inspect_identity(request, &preopt, &repository)?;
    let replay_output = temporary.join("baseline.opt.ll");
    let replay_run = optimize(request, &preopt, &pipeline, &replay_output, &repository)?;
    let mut replay_baseline = retain_outcome(
        &store,
        "baseline/replay",
        &replay_run,
        &source,
        request.line,
    )?;
    for (relative, path, media_type) in [
        (
            "baseline/replay.opt.yaml",
            remark_path(&replay_output),
            "application/yaml",
        ),
        ("baseline/replay.opt.ll", replay_output, "text/plain"),
    ] {
        let artifact = store.retain(relative, &fs::read(path)?, media_type)?;
        replay_baseline.artifacts.push(artifact.clone());
        artifacts.push(artifact);
    }
    artifacts.extend(replay_baseline.artifacts.iter().cloned());

    let mut report = base_report(
        &analysis_id,
        &repository,
        &source,
        source_digest,
        pipeline_digest,
        toolchain,
        subject.clone(),
        request,
        monolithic_baseline,
        replay_baseline,
        &root.join("report.json"),
    );
    if report.monolithic_baseline.classification == "vectorized" {
        report.decline = Some(OptimizationDecline {
            code: "baseline.already_vectorized".to_owned(),
            explanation: "the selected loop already vectorizes in the observed monolithic baseline"
                .to_owned(),
        });
        finalize_report(&store, &mut report, artifacts)?;
        return Ok(report);
    }
    if report.monolithic_baseline.classification != "missed"
        || report.replay_baseline.classification != "missed"
    {
        return Err(OptimizationError::ToolFailure(
            "monolithic and replay baselines do not both observe the selected miss".to_owned(),
        ));
    }

    let ids = report
        .candidates
        .iter()
        .map(|candidate| InterventionId::new(candidate.id.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut cache = BTreeMap::<String, OptimizationExperiment>::new();
    let mut retained_by_variant = Vec::new();
    let limits = SearchLimits {
        max_cardinality: request.max_cardinality.min(ids.len()),
        max_evaluations: request.max_evaluations,
        stop_after_first_successful_cardinality: true,
    };
    let mut fatal = None;
    let search = search_sufficient_sets(ids, limits, |subset| {
        match execute_variant(
            request,
            subset,
            &preopt,
            &pipeline,
            &subject,
            &temporary,
            &repository,
            &store,
        ) {
            Ok((experiment, retained)) => {
                retained_by_variant.extend(retained);
                let verdict = verdict_from_experiment(&experiment);
                cache.insert(subset_key(subset), experiment);
                verdict
            }
            Err(error) => {
                fatal = Some(error);
                ExperimentVerdict::Unresolved(UnresolvedReason::ToolFailed)
            }
        }
    })?;
    if let Some(error) = fatal {
        return Err(error);
    }
    artifacts.extend(retained_by_variant);
    report.experiments = search
        .evaluations()
        .iter()
        .filter_map(|evaluation| cache.remove(&subset_key(evaluation.interventions())))
        .collect();
    minimality_name(search.minimality()).clone_into(&mut report.minimality);
    stop_name(search.stop_reason()).clone_into(&mut report.stop_reason);
    report.declared_subsets = search.declared_subsets();
    if let Some(success) = search.successful_sets().first() {
        let sufficient = success
            .iter()
            .map(|item| item.as_str().to_owned())
            .collect::<Vec<_>>();
        report.finding = Some(OptimizationFinding {
            evidence_strength: "counterfactual_observation".to_owned(),
            summary: format!(
                "Under the recorded toolchain and equivalent-confirmed pipeline, {} was a tested sufficient assumption for the matched loop to vectorize.",
                sufficient.join(" + ")
            ),
            sufficient_assumptions: sufficient,
            minimality: report.minimality.clone(),
            caveats: vec![
                "Parameter-level LLVM noalias is not a source contract or a pairwise range promise.".to_owned(),
                "Repository and obligation analysis are required before any source change.".to_owned(),
            ],
        });
    } else {
        report.decline = Some(OptimizationDecline {
            code: "search.no_successful_assumption".to_owned(),
            explanation: "no evaluated supported assumption changed the matched loop to vectorized"
                .to_owned(),
        });
    }
    finalize_report(&store, &mut report, artifacts)?;
    Ok(report)
}

fn validate_request(request: &OptimizationRequest) -> Result<(), OptimizationError> {
    if request.line == 0 || request.candidates.is_empty() || request.max_evaluations == 0 {
        return Err(OptimizationError::InvalidInput(
            "line, candidates, and evaluation budget must be non-zero".to_owned(),
        ));
    }
    if request.max_cardinality == 0 || request.max_cardinality > request.candidates.len() {
        return Err(OptimizationError::InvalidInput(
            "max cardinality must fit the candidate set".to_owned(),
        ));
    }
    let mut names = request
        .candidates
        .iter()
        .map(|item| item.source_name.as_str())
        .collect::<Vec<_>>();
    names.sort_unstable();
    if names.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(OptimizationError::InvalidInput(
            "duplicate candidate name".to_owned(),
        ));
    }
    Ok(())
}

fn common_clang_arguments(request: &OptimizationRequest, _source: &Path) -> Vec<OsString> {
    vec![
        OsString::from(format!("-{}", request.optimization)),
        OsString::from(format!("-march={}", request.cpu)),
        OsString::from("-gline-tables-only"),
        OsString::from("-gcolumn-info"),
    ]
}

fn capture_toolchain(
    request: &OptimizationRequest,
    current_dir: &Path,
) -> Result<OptimizationToolchain, OptimizationError> {
    Ok(OptimizationToolchain {
        clang: capture_tool(&request.clang, current_dir)?,
        optimizer: capture_tool(&request.optimizer, current_dir)?,
        transformer: capture_tool(&request.transformer, current_dir)?,
        identity_tool: capture_tool(&request.identity_tool, current_dir)?,
        optimization: request.optimization.clone(),
        cpu: request.cpu.clone(),
        normalized_flags: vec![
            format!("-{}", request.optimization),
            format!("-march={}", request.cpu),
            "-gline-tables-only".to_owned(),
            "-gcolumn-info".to_owned(),
        ],
    })
}

fn capture_tool(path: &Path, current_dir: &Path) -> Result<OptimizationTool, OptimizationError> {
    let invocation = resolve_tool(path)?;
    let resolved = invocation.canonicalize()?;
    let result = require_success(
        execute(&invocation, [OsString::from("--version")], current_dir)?,
        "fingerprint optimization tool",
    )?;
    Ok(OptimizationTool {
        invocation_path: invocation.to_string_lossy().into_owned(),
        resolved_path: resolved.to_string_lossy().into_owned(),
        binary_digest: digest(&fs::read(&resolved)?),
        version: String::from_utf8_lossy(&result.stdout).trim().to_owned(),
    })
}

fn resolve_tool(path: &Path) -> Result<PathBuf, OptimizationError> {
    if path.components().count() > 1 {
        return if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            Ok(std::env::current_dir()?.join(path))
        };
    }
    let search = std::env::var_os("PATH")
        .ok_or_else(|| OptimizationError::InvalidInput("PATH is unavailable".to_owned()))?;
    std::env::split_paths(&search)
        .map(|directory| directory.join(path))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| {
            OptimizationError::InvalidInput(format!("tool is unavailable: {}", path.display()))
        })
}

fn execute(
    program: &Path,
    arguments: impl IntoIterator<Item = OsString>,
    current_dir: &Path,
) -> Result<ProcessResult, OptimizationError> {
    let mut request = process_request(program, arguments, current_dir);
    request.timeout = TIMEOUT;
    request.output_limit = OUTPUT_LIMIT;
    Ok(run_process(&request)?)
}

fn require_success(
    result: ProcessResult,
    operation: &str,
) -> Result<ProcessResult, OptimizationError> {
    if result.exit_code == Some(0)
        && !result.timed_out
        && !result.stdout_truncated
        && !result.stderr_truncated
    {
        Ok(result)
    } else {
        Err(OptimizationError::ToolFailure(format!(
            "{operation}: exit={:?}, timeout={}, truncated={}",
            result.exit_code,
            result.timed_out,
            result.stdout_truncated || result.stderr_truncated
        )))
    }
}

fn optimize(
    request: &OptimizationRequest,
    input: &Path,
    pipeline: &str,
    output: &Path,
    current_dir: &Path,
) -> Result<ProcessResult, OptimizationError> {
    let remarks = remark_path(output);
    execute(
        &request.optimizer,
        [
            OsString::from(format!("-passes={pipeline}")),
            OsString::from("-pass-remarks=loop-vectorize"),
            OsString::from("-pass-remarks-missed=loop-vectorize"),
            OsString::from("-pass-remarks-analysis=loop-vectorize"),
            OsString::from(format!("-pass-remarks-output={}", remarks.display())),
            OsString::from("-S"),
            input.as_os_str().to_os_string(),
            OsString::from("-o"),
            output.as_os_str().to_os_string(),
        ],
        current_dir,
    )
}

fn remark_path(optimized_ir: &Path) -> PathBuf {
    optimized_ir.with_extension("remarks.yaml")
}

fn inspect_identity(
    request: &OptimizationRequest,
    input: &Path,
    current_dir: &Path,
) -> Result<LoopIdentity, OptimizationError> {
    let result = require_success(
        execute(
            &request.identity_tool,
            [
                input.as_os_str().to_os_string(),
                OsString::from("--function"),
                OsString::from(&request.function),
                OsString::from("--line"),
                OsString::from(request.line.to_string()),
            ],
            current_dir,
        )?,
        "match LLVM loop identity",
    )?;
    Ok(serde_json::from_slice(&result.stdout)?)
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn execute_variant(
    request: &OptimizationRequest,
    subset: &[InterventionId],
    preopt: &Path,
    pipeline: &str,
    subject: &LoopIdentity,
    temporary: &Path,
    current_dir: &Path,
    store: &ArtifactStore,
) -> Result<(OptimizationExperiment, Vec<ArtifactReference>), OptimizationError> {
    let key = subset_key(subset);
    let experiment_id = format!("exp_{}", &digest(key.as_bytes())[..16]);
    let directory = temporary.join(&experiment_id);
    fs::create_dir(&directory)?;
    let mut input = preopt.to_path_buf();
    let mut retained = Vec::new();
    for (position, assumption) in subset.iter().enumerate() {
        let candidate = request
            .candidates
            .iter()
            .find(|item| candidate_id(item) == assumption.as_str())
            .ok_or_else(|| {
                OptimizationError::InvalidInput("search returned unknown candidate".to_owned())
            })?;
        let output = directory.join(format!("delta-{position}.bc"));
        let transformed = require_success(
            execute(
                &request.transformer,
                [
                    input.as_os_str().to_os_string(),
                    OsString::from("--output"),
                    output.as_os_str().to_os_string(),
                    OsString::from("--function"),
                    OsString::from(&request.function),
                    OsString::from("--parameter-index"),
                    OsString::from(candidate.ir_index.to_string()),
                ],
                current_dir,
            )?,
            "apply typed LLVM intervention",
        )?;
        retained.push(store.retain(
            &format!("variants/{experiment_id}/delta-{position}.json"),
            &transformed.stdout,
            "application/json",
        )?);
        input = output;
    }
    let identity = inspect_identity(request, &input, current_dir)?;
    if &identity != subject {
        return Err(OptimizationError::ToolFailure(
            "loop identity changed before optimization".to_owned(),
        ));
    }
    retained.push(store.retain(
        &format!("variants/{experiment_id}/input.bc"),
        &fs::read(&input)?,
        "application/vnd.llvm.bitcode",
    )?);
    let optimized = directory.join("optimized.ll");
    let run = optimize(request, &input, pipeline, &optimized, current_dir)?;
    let mut outcome = retain_outcome(
        store,
        &format!("variants/{experiment_id}/optimizer"),
        &run,
        &request.source.canonicalize()?,
        request.line,
    )?;
    for (name, path, media_type) in [
        ("optimized.ll", optimized.clone(), "text/plain"),
        ("remarks.yaml", remark_path(&optimized), "application/yaml"),
    ] {
        let artifact = store.retain(
            &format!("variants/{experiment_id}/{name}"),
            &fs::read(path)?,
            media_type,
        )?;
        outcome.artifacts.push(artifact.clone());
        retained.push(artifact);
    }
    if outcome.classification == "vectorized" {
        let confirmation_output = directory.join("confirmation.ll");
        let confirmation_run =
            optimize(request, &input, pipeline, &confirmation_output, current_dir)?;
        let confirmation = retain_outcome(
            store,
            &format!("variants/{experiment_id}/confirmation"),
            &confirmation_run,
            &request.source.canonicalize()?,
            request.line,
        )?;
        let consistent = outcome.classification == confirmation.classification
            && outcome.vector_factor == confirmation.vector_factor
            && outcome.interleave_count == confirmation.interleave_count;
        retained.extend(confirmation.artifacts.iter().cloned());
        outcome.artifacts.extend(confirmation.artifacts);
        for (name, path, media_type) in [
            ("confirmation.ll", confirmation_output.clone(), "text/plain"),
            (
                "confirmation.remarks.yaml",
                remark_path(&confirmation_output),
                "application/yaml",
            ),
        ] {
            let artifact = store.retain(
                &format!("variants/{experiment_id}/{name}"),
                &fs::read(path)?,
                media_type,
            )?;
            outcome.artifacts.push(artifact.clone());
            retained.push(artifact);
        }
        outcome.confirmation_runs = 2;
        outcome.consistent = consistent;
        if !consistent {
            "non_deterministic".clone_into(&mut outcome.classification);
        }
    }
    retained.extend(outcome.artifacts.iter().cloned());
    let verdict = if outcome.classification == "vectorized" {
        "observed"
    } else if outcome.classification == "missed" {
        "not_observed"
    } else {
        "unresolved"
    };
    let unresolved_reason = (verdict == "unresolved").then(|| "tool_failed".to_owned());
    Ok((
        OptimizationExperiment {
            experiment_id,
            assumptions: subset.iter().map(|item| item.as_str().to_owned()).collect(),
            verdict: verdict.to_owned(),
            unresolved_reason,
            ir_verified: true,
            delta_isolated: true,
            loop_identity: identity,
            outcome,
        },
        retained,
    ))
}

fn retain_outcome(
    store: &ArtifactStore,
    prefix: &str,
    run: &ProcessResult,
    source: &Path,
    line: u64,
) -> Result<OptimizationOutcome, OptimizationError> {
    let stdout = store.retain(&format!("{prefix}.stdout"), &run.stdout, "text/plain")?;
    let stderr = store.retain(&format!("{prefix}.stderr"), &run.stderr, "text/plain")?;
    let text = String::from_utf8_lossy(&run.stderr);
    let file = source.to_string_lossy();
    let marker = format!("{file}:{line}:");
    let relative_marker = source
        .file_name()
        .map(|name| format!("{}:{line}:", name.to_string_lossy()));
    let selected = text
        .lines()
        .filter(|item| {
            item.contains(&marker)
                || relative_marker
                    .as_ref()
                    .is_some_and(|relative| item.contains(relative))
        })
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let vectorized = selected
        .iter()
        .find(|item| item.contains("vectorized loop") && !item.contains("not vectorized"));
    let missed = selected
        .iter()
        .any(|item| item.contains("loop not vectorized"));
    let (vector_factor, interleave_count) =
        vectorized.map_or((None, None), |line| parse_vector_widths(line));
    let classification = if run.timed_out {
        "timed_out"
    } else if run.exit_code != Some(0) {
        "tool_failed"
    } else if vectorized.is_some() {
        "vectorized"
    } else if missed {
        "missed"
    } else {
        "loop_absent"
    };
    Ok(OptimizationOutcome {
        classification: classification.to_owned(),
        vector_factor,
        interleave_count,
        selected_remarks: selected,
        exit_code: run.exit_code,
        timed_out: run.timed_out,
        output_truncated: run.stdout_truncated || run.stderr_truncated,
        confirmation_runs: 1,
        consistent: true,
        artifacts: vec![stdout, stderr],
    })
}

fn parse_vector_widths(line: &str) -> (Option<u64>, Option<u64>) {
    (
        number_after(line, "vectorization width: "),
        number_after(line, "interleaved count: "),
    )
}

fn number_after(text: &str, marker: &str) -> Option<u64> {
    let tail = text.split_once(marker)?.1;
    tail.chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()
}

#[allow(clippy::too_many_arguments)]
fn base_report(
    analysis_id: &str,
    repository: &Path,
    source: &Path,
    source_digest: String,
    pipeline_digest: String,
    toolchain: OptimizationToolchain,
    subject: LoopIdentity,
    request: &OptimizationRequest,
    monolithic_baseline: OptimizationOutcome,
    replay_baseline: OptimizationOutcome,
    artifact_path: &Path,
) -> OptimizationReport {
    OptimizationReport {
        schema_version: "2.0.0-dev".to_owned(), analysis_id: analysis_id.to_owned(), query_kind: "optimization_causality".to_owned(), adapter: "clang_llvm".to_owned(), pipeline_fidelity: "equivalent_confirmed".to_owned(), toolchain, repository: repository.to_string_lossy().into_owned(), source: source.to_string_lossy().into_owned(), source_digest, pipeline_digest, subject,
        candidates: request.candidates.iter().map(|candidate| OptimizationCandidateReport { id: candidate_id(candidate), source_name: candidate.source_name.clone(), ir_index: candidate.ir_index, attribute: "noalias".to_owned() }).collect(),
        monolithic_baseline, replay_baseline, experiments: Vec::new(), minimality: "no_successful_set_found".to_owned(), stop_reason: "not_started".to_owned(), declared_subsets: 0, finding: None, decline: None, replay: OptimizationReplay { max_evaluations: request.max_evaluations, max_cardinality: request.max_cardinality }, semantic_digest: String::new(), artifacts: Vec::new(), artifact_path: artifact_path.to_string_lossy().into_owned(), caveats: vec!["Clang's printable pipeline is best-effort; fidelity is equivalent_confirmed, not exact.".to_owned()],
    }
}

fn finalize_report(
    store: &ArtifactStore,
    report: &mut OptimizationReport,
    mut artifacts: Vec<ArtifactReference>,
) -> Result<(), OptimizationError> {
    artifacts.sort();
    artifacts.dedup();
    report.artifacts = artifacts;
    report.semantic_digest = semantic_digest(report)?;
    store.write_new("report.json", &pretty_json(report)?)?;
    store.finalize_read_only()?;
    Ok(())
}

fn semantic_digest(report: &OptimizationReport) -> Result<String, OptimizationError> {
    let mut value = serde_json::to_value(report)?;
    strip_non_semantic_fields(&mut value);
    Ok(digest(&serde_json::to_vec(&value)?))
}

fn strip_non_semantic_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            for field in [
                "analysis_id",
                "artifact_path",
                "artifacts",
                "repository",
                "semantic_digest",
            ] {
                object.remove(field);
            }
            for child in object.values_mut() {
                strip_non_semantic_fields(child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                strip_non_semantic_fields(item);
            }
        }
        _ => {}
    }
}

/// Re-executes a retained optimization-causality report and verifies its semantic projection.
///
/// # Errors
///
/// Returns `OptimizationError` when retained artifacts fail integrity checks, the
/// captured source or toolchain changed, or the replayed optimization observation differs.
pub fn replay_optimization(
    report_path: &Path,
) -> Result<OptimizationReplayResult, OptimizationError> {
    let original: OptimizationReport = serde_json::from_slice(&fs::read(report_path)?)?;
    let root = report_path.parent().ok_or_else(|| {
        OptimizationError::InvalidInput("report path has no parent directory".to_owned())
    })?;
    ArtifactStore::new(root).verify(&original.artifacts)?;
    let recorded_digest = semantic_digest(&original)?;
    if recorded_digest != original.semantic_digest {
        return Err(OptimizationError::Artifact(
            ArtifactError::IntegrityMismatch("report.json semantic contents".to_owned()),
        ));
    }

    let repository = PathBuf::from(&original.repository).canonicalize()?;
    let source = PathBuf::from(&original.source).canonicalize()?;
    let observed_source_digest = digest(&fs::read(&source)?);
    if observed_source_digest != original.source_digest {
        return Err(OptimizationError::ReplayInputChanged {
            expected: original.source_digest,
            observed: observed_source_digest,
        });
    }
    let request = OptimizationRequest {
        repository: repository.clone(),
        source,
        function: original.subject.function.clone(),
        line: original.subject.line,
        candidates: original
            .candidates
            .iter()
            .map(|candidate| ParameterCandidate {
                source_name: candidate.source_name.clone(),
                ir_index: candidate.ir_index,
            })
            .collect(),
        clang: PathBuf::from(&original.toolchain.clang.invocation_path),
        optimizer: PathBuf::from(&original.toolchain.optimizer.invocation_path),
        transformer: PathBuf::from(&original.toolchain.transformer.invocation_path),
        identity_tool: PathBuf::from(&original.toolchain.identity_tool.invocation_path),
        optimization: original.toolchain.optimization.clone(),
        cpu: original.toolchain.cpu.clone(),
        max_evaluations: original.replay.max_evaluations,
        max_cardinality: original.replay.max_cardinality,
    };
    if capture_toolchain(&request, &repository)? != original.toolchain {
        return Err(OptimizationError::ReplayToolchainChanged);
    }
    let replayed = explain_optimization(&request)?;
    if replayed.semantic_digest != original.semantic_digest {
        return Err(OptimizationError::ReplayChanged {
            expected: original.semantic_digest,
            observed: replayed.semantic_digest,
        });
    }
    Ok(OptimizationReplayResult {
        original_analysis_id: original.analysis_id,
        replay_analysis_id: replayed.analysis_id,
        semantic_digest: replayed.semantic_digest,
        matched: true,
    })
}

fn candidate_id(candidate: &ParameterCandidate) -> String {
    format!("parameter.{}.noalias", candidate.source_name)
}

fn pretty_json(value: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}
fn subset_key(subset: &[InterventionId]) -> String {
    subset
        .iter()
        .map(InterventionId::as_str)
        .collect::<Vec<_>>()
        .join("+")
}
fn verdict_from_experiment(experiment: &OptimizationExperiment) -> ExperimentVerdict {
    match experiment.verdict.as_str() {
        "observed" => ExperimentVerdict::Observed,
        "not_observed" => ExperimentVerdict::NotObserved,
        _ => ExperimentVerdict::Unresolved(UnresolvedReason::ToolFailed),
    }
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to String cannot fail");
            output
        })
}
fn analysis_id(source_digest: &str, request: &OptimizationRequest) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let material = format!(
        "{source_digest}:{}:{}:{now}:{}",
        request.function,
        request.line,
        std::process::id()
    );
    format!("wv_{}", &digest(material.as_bytes())[..24])
}
fn minimality_name(value: SearchMinimality) -> &'static str {
    match value {
        SearchMinimality::NoSuccessfulSetFound => "no_successful_set_found",
        SearchMinimality::SmallestSetFound => "smallest_set_found",
        SearchMinimality::MinimalInDeclaredSearch => "minimal_in_declared_search",
        SearchMinimality::UniqueMinimalInDeclaredSearch => "unique_minimal_in_declared_search",
        _ => "unknown_minimality",
    }
}
fn stop_name(value: SearchStopReason) -> &'static str {
    match value {
        SearchStopReason::DeclaredSpaceExhausted => "declared_space_exhausted",
        SearchStopReason::FirstSuccessfulCardinalityCompleted => {
            "first_successful_cardinality_completed"
        }
        SearchStopReason::EvaluationBudgetExhausted => "evaluation_budget_exhausted",
    }
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

    fn request(candidates: Vec<ParameterCandidate>) -> OptimizationRequest {
        OptimizationRequest {
            repository: PathBuf::from("."),
            source: PathBuf::from("fixture.c"),
            function: "kernel".to_owned(),
            line: 3,
            candidates,
            clang: PathBuf::from("clang-21"),
            optimizer: PathBuf::from("opt-21"),
            transformer: PathBuf::from("transform"),
            identity_tool: PathBuf::from("identity"),
            optimization: "O3".to_owned(),
            cpu: "x86-64-v3".to_owned(),
            max_evaluations: 8,
            max_cardinality: 1,
        }
    }

    #[test]
    fn rejects_duplicate_candidate_names_before_tools_run() {
        let candidates = vec![
            ParameterCandidate {
                source_name: "count".to_owned(),
                ir_index: 1,
            },
            ParameterCandidate {
                source_name: "count".to_owned(),
                ir_index: 2,
            },
        ];
        assert!(matches!(
            validate_request(&request(candidates)),
            Err(OptimizationError::InvalidInput(detail)) if detail == "duplicate candidate name"
        ));
    }

    #[test]
    fn parses_vector_width_and_interleave_count_from_observed_remark() {
        assert_eq!(
            parse_vector_widths(
                "remark: kernel.c:5:3: vectorized loop (vectorization width: 8, interleaved count: 4)"
            ),
            (Some(8), Some(4))
        );
    }

    #[test]
    fn candidate_identifier_preserves_assumption_semantics() {
        assert_eq!(
            candidate_id(&ParameterCandidate {
                source_name: "count".to_owned(),
                ir_index: 2,
            }),
            "parameter.count.noalias"
        );
    }
}
