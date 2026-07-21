use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use whyvec_domain::{ExperimentVerdict, SearchMinimality, UnresolvedReason};
use whyvec_experiment::{
    ArtifactError, ArtifactReference, ArtifactStore, InterventionId, ProcessError,
    SearchConfigurationError, SearchLimits, SearchStopReason, process_request, run_process,
    search_sufficient_sets,
};

use crate::diagnostics::{
    DiagnosticRecord, DiagnosticSelectionError, DiagnosticSelector, parse_cargo_json,
    parse_clang_sarif, parse_gcc_json, parse_typescript_json, select_diagnostic,
};
use crate::git::{
    ChangeAtom, ChangeAtomSummary, GitError, GitRepository, SyntaxEditGroup,
    SyntaxEditGroupSummary, TextHunk, TextHunkSummary, apply_text_hunks,
};

const BUILD_TIMEOUT: Duration = Duration::from_mins(3);
const BUILD_OUTPUT_LIMIT: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuildCommand {
    pub program: String,
    pub arguments: Vec<String>,
}

impl BuildCommand {
    #[must_use]
    pub fn cargo(arguments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            program: "cargo".to_owned(),
            arguments: arguments.into_iter().map(Into::into).collect(),
        }
    }

    fn adapter_arguments(&self) -> Result<Vec<OsString>, AnalysisError> {
        let adapter = self.adapter()?;
        let mut arguments = self
            .arguments
            .iter()
            .map(OsString::from)
            .collect::<Vec<_>>();
        match adapter {
            BuildAdapter::CargoRustc => {
                validate_message_format(&self.arguments)?;
                if !has_message_format(&self.arguments) {
                    arguments.push(OsString::from("--message-format=json"));
                }
            }
            BuildAdapter::Clang => {
                reject_compiler_plugins(&self.arguments)?;
                if let Some(format) = diagnostic_format(&self.arguments) {
                    if format != "sarif" {
                        return Err(AnalysisError::UnsupportedDiagnosticFormat {
                            adapter: "clang".to_owned(),
                            format: format.to_owned(),
                        });
                    }
                } else {
                    arguments.push(OsString::from("-fdiagnostics-format=sarif"));
                }
            }
            BuildAdapter::Gcc => {
                reject_compiler_plugins(&self.arguments)?;
                if let Some(format) = diagnostic_format(&self.arguments) {
                    if !matches!(format, "json" | "json-stderr") {
                        return Err(AnalysisError::UnsupportedDiagnosticFormat {
                            adapter: "gcc".to_owned(),
                            format: format.to_owned(),
                        });
                    }
                } else {
                    arguments.push(OsString::from("-fdiagnostics-format=json-stderr"));
                }
            }
            BuildAdapter::TypeScript => {
                if arguments.len() != 1 {
                    return Err(AnalysisError::UnsupportedBuildCommand(
                        "whyvec-typescript requires exactly one tsconfig path".to_owned(),
                    ));
                }
            }
        }
        Ok(arguments)
    }

    fn normalized(&self) -> Result<Self, AnalysisError> {
        Ok(Self {
            program: self.program.clone(),
            arguments: self
                .adapter_arguments()?
                .into_iter()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect(),
        })
    }

    fn adapter(&self) -> Result<BuildAdapter, AnalysisError> {
        if self.program == "cargo" {
            return Ok(BuildAdapter::CargoRustc);
        }
        if self.program == "whyvec-typescript" {
            return Ok(BuildAdapter::TypeScript);
        }
        if Path::new(&self.program).components().count() != 1 {
            return Err(AnalysisError::UnsupportedBuildCommand(self.program.clone()));
        }
        let name = Path::new(&self.program)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if matches!(name, "clang" | "clang++")
            || name.starts_with("clang-")
            || name.starts_with("clang++-")
        {
            return Ok(BuildAdapter::Clang);
        }
        if matches!(name, "gcc" | "g++") || name.starts_with("gcc-") || name.starts_with("g++-") {
            return Ok(BuildAdapter::Gcc);
        }
        Err(AnalysisError::UnsupportedBuildCommand(self.program.clone()))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BuildAdapter {
    CargoRustc,
    Clang,
    Gcc,
    TypeScript,
}

impl BuildAdapter {
    const fn report_name(self) -> &'static str {
        match self {
            Self::CargoRustc => "cargo_rustc",
            Self::Clang => "clang",
            Self::Gcc => "gcc",
            Self::TypeScript => "typescript",
        }
    }
}

#[derive(Clone, Debug)]
pub struct BuildCausalityRequest {
    pub repository: PathBuf,
    pub base: String,
    pub diagnostic: DiagnosticSelector,
    pub command: BuildCommand,
    pub max_evaluations: usize,
    pub max_cardinality: Option<usize>,
    pub max_hunk_evaluations: usize,
    pub max_hunk_cardinality: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuildRunSummary {
    pub subset: Vec<String>,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub output_truncated: bool,
    pub diagnostics: Vec<DiagnosticRecord>,
    pub artifacts: Vec<ArtifactReference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SearchEvaluationSummary {
    pub subset: Vec<String>,
    pub verdict: String,
    pub unresolved_reason: Option<String>,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub output_truncated: bool,
    pub artifacts: Vec<ArtifactReference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolIdentity {
    pub invocation_path: String,
    pub invocation_sha256: String,
    pub resolved_path: String,
    pub resolved_sha256: String,
    pub version: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuildToolchainProvenance {
    pub adapter: String,
    pub sandbox: BuildSandboxProvenance,
    pub tools: Vec<NamedToolIdentity>,
    pub support_files: Vec<SupportFileIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NamedToolIdentity {
    pub role: String,
    pub identity: ToolIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SupportFileIdentity {
    pub role: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuildSandboxProvenance {
    pub provider: String,
    pub tool: ToolIdentity,
    pub network_isolated: bool,
    pub host_root_read_only: bool,
    pub private_tmp: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplaySpecification {
    pub input_digest: String,
    pub command_digest: String,
    pub max_evaluations: usize,
    pub max_cardinality: usize,
    pub max_hunk_evaluations: usize,
    pub max_hunk_cardinality: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplayResult {
    pub original_analysis_id: String,
    pub replay_analysis_id: String,
    pub semantic_digest: String,
    pub matched: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CausalSetReport {
    pub sufficient_atoms: Vec<String>,
    pub sufficient_files: Vec<String>,
    pub removal_subset: Vec<String>,
    pub target_removed_from_full_patch: bool,
    pub diagnostics_suppressed_with_target: Vec<DiagnosticRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HunkCausalSetReport {
    pub sufficient_groups: Vec<String>,
    pub sufficient_hunks: Vec<String>,
    pub locations: Vec<String>,
    pub removal_file_atoms: Vec<String>,
    pub removal_groups: Vec<String>,
    pub removal_hunks: Vec<String>,
    pub target_removed_from_full_patch: bool,
    pub diagnostics_suppressed_with_target: Vec<DiagnosticRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HunkRefinementReport {
    pub parent_sufficient_atoms: Vec<String>,
    pub fixed_atoms: Vec<String>,
    pub hunks: Vec<TextHunkSummary>,
    pub grouping: String,
    pub syntax_groups: Vec<SyntaxEditGroupSummary>,
    pub evaluations: Vec<SearchEvaluationSummary>,
    pub minimality: String,
    pub stop_reason: String,
    pub declared_subsets: usize,
    pub causal_sets: Vec<HunkCausalSetReport>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuildCausalityReport {
    pub schema_version: String,
    pub analysis_id: String,
    pub query_kind: String,
    pub adapter: String,
    pub evidence_strength: String,
    pub repository: String,
    pub base_commit: String,
    pub input_digest: String,
    pub command_digest: String,
    pub toolchain: BuildToolchainProvenance,
    pub command: BuildCommand,
    pub target_diagnostic: DiagnosticRecord,
    pub atoms: Vec<ChangeAtomSummary>,
    pub baseline: BuildRunSummary,
    pub candidate: BuildRunSummary,
    pub evaluations: Vec<SearchEvaluationSummary>,
    pub minimality: String,
    pub stop_reason: String,
    pub declared_subsets: usize,
    pub causal_sets: Vec<CausalSetReport>,
    pub hunk_refinements: Vec<HunkRefinementReport>,
    pub semantic_digest: String,
    pub replay: ReplaySpecification,
    pub artifacts: Vec<ArtifactReference>,
    pub artifact_path: String,
    pub caveats: Vec<String>,
}

#[derive(Debug)]
pub enum AnalysisError {
    Git(GitError),
    Process(ProcessError),
    Search(SearchConfigurationError),
    DiagnosticSelection(DiagnosticSelectionError),
    Io(std::io::Error),
    Json(serde_json::Error),
    Artifact(ArtifactError),
    NoChanges,
    BaselineFailed(Vec<DiagnosticRecord>),
    CandidateSucceeded,
    UnsupportedBuildCommand(String),
    UnsupportedMessageFormat(String),
    UnsupportedDiagnosticFormat { adapter: String, format: String },
    InterventionId(String),
    MissingCachedEvaluation(Vec<String>),
    RefinementDidNotReproduce(Vec<String>),
    ArtifactIntegrity(String),
    ReplayInputChanged { expected: String, observed: String },
    ReplayToolchainChanged,
    ReplayOutcomeChanged { expected: String, observed: String },
    WorktreeCleanup { path: PathBuf, source: GitError },
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Git(error) => error.fmt(formatter),
            Self::Process(error) => error.fmt(formatter),
            Self::Search(error) => error.fmt(formatter),
            Self::DiagnosticSelection(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
            Self::Json(error) => error.fmt(formatter),
            Self::Artifact(error) => write!(formatter, "artifact integrity check failed: {error}"),
            Self::NoChanges => formatter.write_str("no tracked or untracked changes were found"),
            Self::BaselineFailed(diagnostics) => write!(
                formatter,
                "the base revision does not pass the selected build command ({} diagnostics)",
                diagnostics.len()
            ),
            Self::CandidateSucceeded => formatter.write_str(
                "the full working-tree change passes; there is no failing diagnostic to explain",
            ),
            Self::UnsupportedBuildCommand(program) => {
                write!(formatter, "unsupported build adapter for program {program}")
            }
            Self::UnsupportedMessageFormat(format) => write!(
                formatter,
                "Cargo message format must be JSON for diagnostic evidence, received {format}"
            ),
            Self::UnsupportedDiagnosticFormat { adapter, format } => write!(
                formatter,
                "{adapter} diagnostic format must be structured JSON or SARIF, received {format}"
            ),
            Self::InterventionId(identifier) => {
                write!(formatter, "invalid generated intervention id: {identifier}")
            }
            Self::MissingCachedEvaluation(subset) => write!(
                formatter,
                "search retained an evaluation without a cached build run: {subset:?}"
            ),
            Self::RefinementDidNotReproduce(atoms) => write!(
                formatter,
                "zero-context hunk reconstruction did not reproduce sufficient file set {atoms:?}"
            ),
            Self::ArtifactIntegrity(detail) => {
                write!(formatter, "artifact integrity check failed: {detail}")
            }
            Self::ReplayInputChanged { expected, observed } => write!(
                formatter,
                "replay input digest changed (expected {expected}, observed {observed})"
            ),
            Self::ReplayToolchainChanged => formatter.write_str(
                "replay toolchain fingerprint differs from the recorded adapter toolchain",
            ),
            Self::ReplayOutcomeChanged { expected, observed } => write!(
                formatter,
                "replay semantic digest changed (expected {expected}, observed {observed})"
            ),
            Self::WorktreeCleanup { path, source } => write!(
                formatter,
                "failed to remove isolated worktree {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for AnalysisError {}

impl From<GitError> for AnalysisError {
    fn from(value: GitError) -> Self {
        Self::Git(value)
    }
}

impl From<ProcessError> for AnalysisError {
    fn from(value: ProcessError) -> Self {
        Self::Process(value)
    }
}

impl From<SearchConfigurationError> for AnalysisError {
    fn from(value: SearchConfigurationError) -> Self {
        Self::Search(value)
    }
}

impl From<DiagnosticSelectionError> for AnalysisError {
    fn from(value: DiagnosticSelectionError) -> Self {
        Self::DiagnosticSelection(value)
    }
}

impl From<std::io::Error> for AnalysisError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for AnalysisError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<ArtifactError> for AnalysisError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

/// Executes a build-causality query in isolated detached worktrees.
///
/// # Errors
///
/// Returns `AnalysisError` when repository capture, baseline reproduction,
/// diagnostic selection, a compiler experiment, or report persistence fails.
#[allow(clippy::too_many_lines)]
pub fn explain_build(
    request: &BuildCausalityRequest,
) -> Result<BuildCausalityReport, AnalysisError> {
    let repository = GitRepository::discover(&request.repository, &request.base)?;
    let atoms = repository.capture_atoms()?;
    if atoms.is_empty() {
        return Err(AnalysisError::NoChanges);
    }
    let analysis_id = analysis_id(&repository, &atoms);
    let artifact_parent = repository.root.join(".whyvec").join("analyses");
    let artifact_root = artifact_parent.join(&analysis_id);
    fs::create_dir_all(&artifact_parent)?;
    fs::create_dir(&artifact_root)?;
    let command = request.command.normalized()?;
    let adapter = command.adapter()?;
    let toolchain = capture_toolchain(&repository.root, &command)?;
    let (driver_path, driver_prefix) = execution_driver(&toolchain)?;
    let input_digest = input_digest(&repository, &atoms);
    let command_digest = command_digest(&command, &toolchain);
    let artifact_store = ArtifactStore::new(&artifact_root);
    let mut retained_artifacts = Vec::new();
    for atom in &atoms {
        retained_artifacts.push(artifact_store.retain(
            &format!("inputs/atoms/{}.bin", atom.id),
            &atom.captured_bytes(),
            "application/octet-stream",
        )?);
    }
    let provenance = serde_json::to_vec_pretty(&serde_json::json!({
        "base_commit": repository.base_commit,
        "input_digest": input_digest,
        "command_digest": command_digest,
        "command": command,
        "toolchain": toolchain,
        "atoms": atoms.iter().map(ChangeAtomSummary::from).collect::<Vec<_>>(),
    }))?;
    retained_artifacts.push(artifact_store.retain(
        "inputs/provenance.json",
        &provenance,
        "application/json",
    )?);
    let temporary_root = std::env::temp_dir().join(format!("whyvec-{analysis_id}"));
    fs::create_dir(&temporary_root)?;
    let _temporary_guard = DirectoryCleanup(temporary_root.clone());
    let target_dir = temporary_root.join("build-output");
    fs::create_dir_all(&target_dir)?;

    let mut session = AnalysisSession {
        repository,
        atoms,
        command: command.clone(),
        adapter,
        driver_path,
        driver_prefix,
        sandbox_path: PathBuf::from(&toolchain.sandbox.tool.invocation_path),
        target_dir,
        worktree_root: temporary_root.join("worktrees"),
        artifact_root: artifact_root.clone(),
        next_worktree: 0,
        cache: BTreeMap::new(),
        artifacts: retained_artifacts,
    };
    fs::create_dir_all(&session.worktree_root)?;

    let baseline = session.evaluate(&[])?;
    if baseline.exit_code != Some(0) || baseline.timed_out {
        return Err(AnalysisError::BaselineFailed(baseline.diagnostics));
    }

    let all_ids = session
        .atoms
        .iter()
        .map(|atom| atom.id.clone())
        .collect::<Vec<_>>();
    let candidate = session.evaluate(&all_ids)?;
    if candidate.exit_code == Some(0) && !candidate.timed_out {
        return Err(AnalysisError::CandidateSucceeded);
    }
    let target = select_diagnostic(&candidate.diagnostics, &request.diagnostic)?;

    let intervention_ids = session
        .atoms
        .iter()
        .map(|atom| {
            InterventionId::new(atom.id.clone())
                .map_err(|_| AnalysisError::InterventionId(atom.id.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let max_cardinality = request
        .max_cardinality
        .unwrap_or(intervention_ids.len())
        .min(intervention_ids.len());
    let limits = SearchLimits {
        max_cardinality,
        max_evaluations: request.max_evaluations,
        stop_after_first_successful_cardinality: true,
    };

    let mut oracle_failure = None;
    let search = search_sufficient_sets(intervention_ids, limits, |subset| {
        let ids = subset
            .iter()
            .map(|identifier| identifier.as_str().to_owned())
            .collect::<Vec<_>>();
        match session.evaluate(&ids) {
            Ok(run) => classify_run(&run, &target.id),
            Err(error) if is_invalid_intervention(&error) => {
                ExperimentVerdict::Unresolved(UnresolvedReason::InterventionInvalid)
            }
            Err(error) => {
                oracle_failure = Some(error);
                ExperimentVerdict::Unresolved(UnresolvedReason::ToolFailed)
            }
        }
    })?;
    if let Some(error) = oracle_failure {
        return Err(error);
    }

    let atom_map = session
        .atoms
        .iter()
        .map(|atom| (atom.id.clone(), atom.paths.clone()))
        .collect::<BTreeMap<_, _>>();
    let candidate_ids = candidate
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut causal_sets = Vec::new();
    for sufficient in search.successful_sets() {
        let sufficient_ids = sufficient
            .iter()
            .map(|identifier| identifier.as_str().to_owned())
            .collect::<Vec<_>>();
        let sufficient_lookup = sufficient_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let complement = all_ids
            .iter()
            .filter(|identifier| !sufficient_lookup.contains(identifier.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        let removal = session.evaluate(&complement)?;
        let removal_ids = removal
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<BTreeSet<_>>();
        let suppressed = candidate
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                candidate_ids.contains(diagnostic.id.as_str())
                    && !removal_ids.contains(diagnostic.id.as_str())
            })
            .cloned()
            .collect::<Vec<_>>();
        let files = sufficient_ids
            .iter()
            .filter_map(|identifier| atom_map.get(identifier))
            .flat_map(Clone::clone)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        causal_sets.push(CausalSetReport {
            sufficient_atoms: sufficient_ids,
            sufficient_files: files,
            removal_subset: complement,
            target_removed_from_full_patch: !removal_ids.contains(target.id.as_str()),
            diagnostics_suppressed_with_target: suppressed,
        });
    }

    let mut hunk_refinements = Vec::new();
    for causal_set in &causal_sets {
        if let Some(refinement) = refine_hunks(
            &mut session,
            causal_set,
            &all_ids,
            &candidate,
            &target,
            request,
        )? {
            hunk_refinements.push(refinement);
        }
    }

    let evaluations = search
        .evaluations()
        .iter()
        .map(|evaluation| {
            let (verdict, unresolved_reason) = verdict_strings(evaluation.verdict());
            let subset = evaluation
                .interventions()
                .iter()
                .map(|identifier| identifier.as_str().to_owned())
                .collect::<Vec<_>>();
            evaluation_summary(&session, subset, verdict, unresolved_reason)
        })
        .collect::<Result<Vec<_>, AnalysisError>>()?;
    let artifact_path = artifact_root.join("report.json");
    let replay = ReplaySpecification {
        input_digest: input_digest.clone(),
        command_digest: command_digest.clone(),
        max_evaluations: request.max_evaluations,
        max_cardinality,
        max_hunk_evaluations: request.max_hunk_evaluations,
        max_hunk_cardinality: request.max_hunk_cardinality.unwrap_or_else(|| {
            hunk_refinements
                .iter()
                .map(|refinement| refinement.syntax_groups.len())
                .max()
                .unwrap_or(1)
        }),
    };
    let mut report = BuildCausalityReport {
        schema_version: "2.0.0-dev".to_owned(),
        analysis_id,
        query_kind: "build_causality".to_owned(),
        adapter: adapter.report_name().to_owned(),
        evidence_strength: "counterfactual_observation".to_owned(),
        repository: session.repository.root.to_string_lossy().into_owned(),
        base_commit: session.repository.base_commit.clone(),
        input_digest,
        command_digest,
        toolchain,
        command,
        target_diagnostic: target,
        atoms: session.atoms.iter().map(ChangeAtomSummary::from).collect(),
        baseline,
        candidate,
        evaluations,
        minimality: minimality_name(search.minimality()).to_owned(),
        stop_reason: stop_reason_name(search.stop_reason()).to_owned(),
        declared_subsets: search.declared_subsets(),
        causal_sets,
        hunk_refinements,
        semantic_digest: String::new(),
        replay,
        artifacts: session.artifacts.clone(),
        artifact_path: artifact_path.to_string_lossy().into_owned(),
        caveats: vec![
            "A sufficient edit set changes the selected compiler observation; it does not prove the edit is semantically wrong.".to_owned(),
            "Tracked text files are refined to zero-context Git hunks; these are executable edit regions, not syntax-tree-level semantic units.".to_owned(),
            "Compiler runs use a Bubblewrap mount, process, and network sandbox; the host root is read-only and only the fresh worktree, build-output directory, and private temporary filesystem are writable.".to_owned(),
        ],
    };
    if report.candidate.output_truncated
        || report
            .evaluations
            .iter()
            .any(|evaluation| evaluation.output_truncated)
    {
        report.caveats.push(
            "At least one compiler output exceeded the retained bound; absence and cascade comparisons for that run are incomplete.".to_owned(),
        );
    }
    let cleanup = fs::remove_dir_all(&temporary_root);
    if let Err(error) = cleanup {
        report.caveats.push(format!(
            "temporary analysis cleanup failed at {}: {error}",
            temporary_root.display()
        ));
    }
    report.artifacts.sort();
    report.artifacts.dedup();
    report.semantic_digest = semantic_digest(&report)?;
    let serialized = serde_json::to_vec_pretty(&report)?;
    artifact_store.write_new("report.json", &serialized)?;
    artifact_store.finalize_read_only()?;
    Ok(report)
}

struct AnalysisSession {
    repository: GitRepository,
    atoms: Vec<ChangeAtom>,
    command: BuildCommand,
    adapter: BuildAdapter,
    driver_path: PathBuf,
    driver_prefix: Vec<OsString>,
    sandbox_path: PathBuf,
    target_dir: PathBuf,
    worktree_root: PathBuf,
    artifact_root: PathBuf,
    next_worktree: usize,
    cache: BTreeMap<String, BuildRunSummary>,
    artifacts: Vec<ArtifactReference>,
}

struct DirectoryCleanup(PathBuf);

impl Drop for DirectoryCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl AnalysisSession {
    fn cached(&self, identifiers: &[String]) -> Option<&BuildRunSummary> {
        let mut sorted = identifiers.to_vec();
        sorted.sort();
        self.cache.get(&sorted.join("\u{1f}"))
    }

    fn evaluate(&mut self, identifiers: &[String]) -> Result<BuildRunSummary, AnalysisError> {
        self.evaluate_variant(identifiers, &[])
    }

    fn evaluate_variant(
        &mut self,
        file_identifiers: &[String],
        hunks: &[TextHunk],
    ) -> Result<BuildRunSummary, AnalysisError> {
        let mut identifiers = file_identifiers.to_vec();
        identifiers.extend(hunks.iter().map(|hunk| hunk.summary.id.clone()));
        let mut sorted = identifiers.clone();
        sorted.sort();
        let key = sorted.join("\u{1f}");
        if let Some(cached) = self.cache.get(&key) {
            return Ok(cached.clone());
        }

        let worktree = self
            .worktree_root
            .join(format!("evaluation-{:05}", self.next_worktree));
        self.next_worktree += 1;
        self.repository.add_worktree(&worktree)?;

        let evaluation = self.evaluate_in_worktree(file_identifiers, hunks, &sorted, &worktree);
        let cleanup = self.repository.remove_worktree(&worktree);
        if let Err(source) = cleanup {
            return Err(AnalysisError::WorktreeCleanup {
                path: worktree,
                source,
            });
        }
        let summary = evaluation?;
        self.artifacts.extend(summary.artifacts.iter().cloned());
        self.cache.insert(key, summary.clone());
        Ok(summary)
    }

    fn syntax_groups(
        &mut self,
        file_identifiers: &[String],
        hunks: &[TextHunk],
    ) -> Result<Vec<SyntaxEditGroup>, AnalysisError> {
        let worktree = self
            .worktree_root
            .join(format!("grouping-{:05}", self.next_worktree));
        self.next_worktree += 1;
        self.repository.add_worktree(&worktree)?;
        let files = hunks
            .iter()
            .map(|hunk| hunk.summary.file.clone())
            .collect::<BTreeSet<_>>();
        let old_sources = read_text_sources(&worktree, &files);
        let selected = file_identifiers
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let grouping = (|| {
            for atom in &self.atoms {
                if selected.contains(atom.id.as_str()) {
                    atom.apply(&worktree)?;
                }
            }
            apply_text_hunks(hunks, &worktree)?;
            let new_sources = read_text_sources(&worktree, &files);
            Ok(crate::syntax::group_hunks(
                hunks,
                &old_sources,
                &new_sources,
            ))
        })();
        let cleanup = self.repository.remove_worktree(&worktree);
        if let Err(source) = cleanup {
            return Err(AnalysisError::WorktreeCleanup {
                path: worktree,
                source,
            });
        }
        grouping
    }

    fn evaluate_in_worktree(
        &self,
        file_identifiers: &[String],
        hunks: &[TextHunk],
        identifiers: &[String],
        worktree: &Path,
    ) -> Result<BuildRunSummary, AnalysisError> {
        let selected = file_identifiers
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        for atom in &self.atoms {
            if selected.contains(atom.id.as_str()) {
                atom.apply(worktree)?;
            }
        }
        apply_text_hunks(hunks, worktree)?;

        let mut sandbox_arguments = vec![
            OsString::from("--die-with-parent"),
            OsString::from("--new-session"),
            OsString::from("--unshare-all"),
            OsString::from("--ro-bind"),
            OsString::from("/"),
            OsString::from("/"),
            OsString::from("--dev"),
            OsString::from("/dev"),
            OsString::from("--proc"),
            OsString::from("/proc"),
            OsString::from("--tmpfs"),
            OsString::from("/tmp"),
            OsString::from("--bind"),
            worktree.as_os_str().to_os_string(),
            worktree.as_os_str().to_os_string(),
            OsString::from("--bind"),
            self.target_dir.as_os_str().to_os_string(),
            self.target_dir.as_os_str().to_os_string(),
            OsString::from("--chdir"),
            worktree.as_os_str().to_os_string(),
            OsString::from("--"),
            self.driver_path.as_os_str().to_os_string(),
        ];
        sandbox_arguments.extend(self.driver_prefix.iter().cloned());
        sandbox_arguments.extend(self.command.adapter_arguments()?);
        let mut process_request =
            process_request(&self.sandbox_path, sandbox_arguments, Path::new("/"));
        process_request.timeout = BUILD_TIMEOUT;
        process_request.output_limit = BUILD_OUTPUT_LIMIT;
        if self.adapter == BuildAdapter::CargoRustc {
            process_request.environment.extend([
                (
                    OsString::from("CARGO_TARGET_DIR"),
                    self.target_dir.as_os_str().to_os_string(),
                ),
                (OsString::from("CARGO_NET_OFFLINE"), OsString::from("true")),
                (OsString::from("CARGO_TERM_COLOR"), OsString::from("never")),
            ]);
        }
        let result = run_process(&process_request)?;
        let diagnostics = match self.adapter {
            BuildAdapter::CargoRustc => parse_cargo_json(&result.stdout, worktree),
            BuildAdapter::Clang => parse_clang_sarif(&result.stderr, worktree),
            BuildAdapter::Gcc => parse_gcc_json(&result.stderr, worktree),
            BuildAdapter::TypeScript => parse_typescript_json(&result.stdout, worktree),
        };
        let run_name = run_artifact_name(identifiers);
        let artifact_store = ArtifactStore::new(&self.artifact_root);
        let stdout = artifact_store.retain(
            &format!("runs/{run_name}/stdout.bin"),
            &result.stdout,
            "application/octet-stream",
        )?;
        let stderr = artifact_store.retain(
            &format!("runs/{run_name}/stderr.bin"),
            &result.stderr,
            "application/octet-stream",
        )?;
        Ok(BuildRunSummary {
            subset: identifiers.to_vec(),
            exit_code: result.exit_code,
            timed_out: result.timed_out,
            output_truncated: result.stdout_truncated || result.stderr_truncated,
            diagnostics,
            artifacts: vec![stdout, stderr],
        })
    }
}

fn read_text_sources(worktree: &Path, files: &BTreeSet<String>) -> BTreeMap<String, String> {
    files
        .iter()
        .filter_map(|file| {
            fs::read_to_string(worktree.join(file))
                .ok()
                .map(|source| (file.clone(), source))
        })
        .collect()
}

#[allow(clippy::too_many_lines)]
fn refine_hunks(
    session: &mut AnalysisSession,
    parent: &CausalSetReport,
    all_file_ids: &[String],
    candidate: &BuildRunSummary,
    target: &DiagnosticRecord,
    request: &BuildCausalityRequest,
) -> Result<Option<HunkRefinementReport>, AnalysisError> {
    let parent_ids = parent
        .sufficient_atoms
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut fixed_atoms = Vec::new();
    let mut hunks = Vec::new();
    for atom in session
        .atoms
        .iter()
        .filter(|atom| parent_ids.contains(&atom.id))
    {
        let atom_hunks = atom.text_hunks();
        if atom_hunks.is_empty() {
            fixed_atoms.push(atom.id.clone());
        } else {
            hunks.extend(atom_hunks);
        }
    }
    if hunks.is_empty() {
        return Ok(None);
    }
    let full_hunk_run = session.evaluate_variant(&fixed_atoms, &hunks)?;
    if classify_run(&full_hunk_run, &target.id) != ExperimentVerdict::Observed {
        return Err(AnalysisError::RefinementDidNotReproduce(
            parent.sufficient_atoms.clone(),
        ));
    }
    let groups = session.syntax_groups(&fixed_atoms, &hunks)?;
    let candidates = groups
        .iter()
        .map(|group| {
            InterventionId::new(group.summary.id.clone())
                .map_err(|_| AnalysisError::InterventionId(group.summary.id.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let max_cardinality = request
        .max_hunk_cardinality
        .unwrap_or(candidates.len())
        .min(candidates.len());
    let limits = SearchLimits {
        max_cardinality,
        max_evaluations: request.max_hunk_evaluations,
        stop_after_first_successful_cardinality: true,
    };
    let group_map = groups
        .iter()
        .map(|group| (group.summary.id.clone(), group.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut oracle_failure = None;
    let search = search_sufficient_sets(candidates, limits, |subset| {
        let selected = subset
            .iter()
            .filter_map(|id| group_map.get(id.as_str()))
            .flat_map(|group| group.hunks.iter().cloned())
            .collect::<Vec<_>>();
        match session.evaluate_variant(&fixed_atoms, &selected) {
            Ok(run) => classify_run(&run, &target.id),
            Err(error) if is_invalid_intervention(&error) => {
                ExperimentVerdict::Unresolved(UnresolvedReason::InterventionInvalid)
            }
            Err(error) => {
                oracle_failure = Some(error);
                ExperimentVerdict::Unresolved(UnresolvedReason::InterventionInvalid)
            }
        }
    })?;
    if let Some(error) = oracle_failure {
        return Err(error);
    }
    let candidate_ids = candidate
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<BTreeSet<_>>();
    let outside_parent = all_file_ids
        .iter()
        .filter(|id| !parent_ids.contains(*id))
        .cloned()
        .chain(fixed_atoms.iter().cloned())
        .collect::<Vec<_>>();
    let mut causal_sets = Vec::new();
    for sufficient in search.successful_sets() {
        let sufficient_groups = sufficient
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let sufficient_hunks = sufficient_groups
            .iter()
            .filter_map(|id| group_map.get(id))
            .flat_map(|group| group.hunks.iter().map(|hunk| hunk.summary.id.clone()))
            .collect::<BTreeSet<_>>();
        let complement_groups = groups
            .iter()
            .filter(|group| !sufficient_groups.contains(&group.summary.id))
            .collect::<Vec<_>>();
        let complement = complement_groups
            .iter()
            .flat_map(|group| group.hunks.iter().cloned())
            .collect::<Vec<_>>();
        let removal = session.evaluate_variant(&outside_parent, &complement)?;
        let removal_ids = removal
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<BTreeSet<_>>();
        causal_sets.push(HunkCausalSetReport {
            sufficient_groups: sufficient_groups.iter().cloned().collect(),
            sufficient_hunks: sufficient_hunks.iter().cloned().collect(),
            locations: sufficient_groups
                .iter()
                .filter_map(|id| group_map.get(id))
                .map(|group| {
                    let line = group.hunks.first().map_or(0, |hunk| hunk.summary.new_start);
                    group.summary.symbol.as_ref().map_or_else(
                        || format!("{}:{line}", group.summary.file),
                        |symbol| format!("{}:{line} ({symbol})", group.summary.file),
                    )
                })
                .collect(),
            removal_file_atoms: outside_parent.clone(),
            removal_groups: complement_groups
                .iter()
                .map(|group| group.summary.id.clone())
                .collect(),
            removal_hunks: complement
                .iter()
                .map(|hunk| hunk.summary.id.clone())
                .collect(),
            target_removed_from_full_patch: !removal_ids.contains(target.id.as_str()),
            diagnostics_suppressed_with_target: candidate
                .diagnostics
                .iter()
                .filter(|diagnostic| {
                    candidate_ids.contains(diagnostic.id.as_str())
                        && !removal_ids.contains(diagnostic.id.as_str())
                })
                .cloned()
                .collect(),
        });
    }
    let evaluations = search
        .evaluations()
        .iter()
        .map(|evaluation| {
            let subset = evaluation
                .interventions()
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect::<Vec<_>>();
            let mut cache_ids = fixed_atoms.clone();
            cache_ids.extend(
                subset
                    .iter()
                    .filter_map(|id| group_map.get(id))
                    .flat_map(|group| group.hunks.iter().map(|hunk| hunk.summary.id.clone())),
            );
            let (verdict, unresolved_reason) = verdict_strings(evaluation.verdict());
            let mut summary = evaluation_summary(session, cache_ids, verdict, unresolved_reason)?;
            summary.subset = subset;
            Ok(summary)
        })
        .collect::<Result<Vec<_>, AnalysisError>>()?;
    Ok(Some(HunkRefinementReport {
        parent_sufficient_atoms: parent.sufficient_atoms.clone(),
        fixed_atoms,
        hunks: hunks.iter().map(|hunk| hunk.summary.clone()).collect(),
        grouping: if groups.iter().all(|group| group.summary.language == "rust") {
            "rust_item".to_owned()
        } else {
            "text_hunk_fallback".to_owned()
        },
        syntax_groups: groups.iter().map(|group| group.summary.clone()).collect(),
        evaluations,
        minimality: minimality_name(search.minimality()).to_owned(),
        stop_reason: stop_reason_name(search.stop_reason()).to_owned(),
        declared_subsets: search.declared_subsets(),
        causal_sets,
    }))
}

fn is_invalid_intervention(error: &AnalysisError) -> bool {
    matches!(
        error,
        AnalysisError::Git(GitError::CommandFailed {
            operation: "apply" | "apply refined hunks",
            ..
        })
    )
}

fn evaluation_summary(
    session: &AnalysisSession,
    subset: Vec<String>,
    verdict: &str,
    unresolved_reason: Option<&str>,
) -> Result<SearchEvaluationSummary, AnalysisError> {
    if let Some(run) = session.cached(&subset) {
        return Ok(SearchEvaluationSummary {
            subset,
            verdict: verdict.to_owned(),
            unresolved_reason: unresolved_reason.map(str::to_owned),
            exit_code: run.exit_code,
            timed_out: run.timed_out,
            output_truncated: run.output_truncated,
            artifacts: run.artifacts.clone(),
        });
    }
    if unresolved_reason == Some("intervention_invalid") {
        return Ok(SearchEvaluationSummary {
            subset,
            verdict: verdict.to_owned(),
            unresolved_reason: unresolved_reason.map(str::to_owned),
            exit_code: None,
            timed_out: false,
            output_truncated: false,
            artifacts: Vec::new(),
        });
    }
    Err(AnalysisError::MissingCachedEvaluation(subset))
}

fn classify_run(run: &BuildRunSummary, target_id: &str) -> ExperimentVerdict {
    if run.timed_out {
        return ExperimentVerdict::Unresolved(UnresolvedReason::TimedOut);
    }
    if run
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.id == target_id)
    {
        return ExperimentVerdict::Observed;
    }
    if run.exit_code == Some(0) {
        ExperimentVerdict::NotObserved
    } else {
        ExperimentVerdict::Unresolved(UnresolvedReason::CompileFailedForDifferentReason)
    }
}

fn has_message_format(arguments: &[String]) -> bool {
    arguments
        .iter()
        .any(|argument| argument == "--message-format" || argument.starts_with("--message-format="))
}

fn validate_message_format(arguments: &[String]) -> Result<(), AnalysisError> {
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        let format = if argument == "--message-format" {
            index += 1;
            arguments
                .get(index)
                .ok_or_else(|| AnalysisError::UnsupportedMessageFormat("<missing>".to_owned()))?
                .as_str()
        } else if let Some(format) = argument.strip_prefix("--message-format=") {
            format
        } else {
            index += 1;
            continue;
        };
        if !format.starts_with("json") {
            return Err(AnalysisError::UnsupportedMessageFormat(format.to_owned()));
        }
        index += 1;
    }
    Ok(())
}

fn diagnostic_format(arguments: &[String]) -> Option<&str> {
    arguments
        .iter()
        .find_map(|argument| argument.strip_prefix("-fdiagnostics-format="))
}

fn reject_compiler_plugins(arguments: &[String]) -> Result<(), AnalysisError> {
    let denied = arguments.iter().find(|argument| {
        matches!(argument.as_str(), "-Xclang" | "-load" | "-load-pass-plugin")
            || argument.starts_with('@')
            || argument.starts_with("-fplugin")
            || argument.starts_with("-fpass-plugin")
            || argument.starts_with("-load-pass-plugin=")
            || argument.starts_with("-specs")
            || argument.starts_with("-wrapper")
            || argument == &"-B"
            || (argument.starts_with("-B") && argument.len() > 2)
            || argument.starts_with("--config")
    });
    if let Some(argument) = denied {
        return Err(AnalysisError::UnsupportedBuildCommand(format!(
            "compiler plugin argument is denied: {argument}"
        )));
    }
    Ok(())
}

fn verdict_strings(verdict: ExperimentVerdict) -> (&'static str, Option<&'static str>) {
    match verdict {
        ExperimentVerdict::Observed => ("observed", None),
        ExperimentVerdict::NotObserved => ("not_observed", None),
        ExperimentVerdict::Unresolved(reason) => ("unresolved", Some(unresolved_name(reason))),
    }
}

fn unresolved_name(reason: UnresolvedReason) -> &'static str {
    match reason {
        UnresolvedReason::SubjectAbsent => "subject_absent",
        UnresolvedReason::SubjectAmbiguous => "subject_ambiguous",
        UnresolvedReason::InterventionInvalid => "intervention_invalid",
        UnresolvedReason::CompileFailedForDifferentReason => "compile_failed_for_different_reason",
        UnresolvedReason::ToolFailed => "tool_failed",
        UnresolvedReason::TimedOut => "timed_out",
        UnresolvedReason::PolicyDenied => "policy_denied",
        UnresolvedReason::NonDeterministic => "non_deterministic",
        _ => "unknown_unresolved_reason",
    }
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

fn stop_reason_name(value: SearchStopReason) -> &'static str {
    match value {
        SearchStopReason::DeclaredSpaceExhausted => "declared_space_exhausted",
        SearchStopReason::FirstSuccessfulCardinalityCompleted => {
            "first_successful_cardinality_completed"
        }
        SearchStopReason::EvaluationBudgetExhausted => "evaluation_budget_exhausted",
    }
}

fn analysis_id(repository: &GitRepository, atoms: &[ChangeAtom]) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut digest = Sha256::new();
    digest.update(repository.base_commit.as_bytes());
    digest.update(std::process::id().to_le_bytes());
    digest.update(now.to_le_bytes());
    for atom in atoms {
        digest.update(atom.id.as_bytes());
    }
    let bytes = digest.finalize();
    let short = crate::hex_prefix(&bytes, 12);
    format!("wv_{short}")
}

fn input_digest(repository: &GitRepository, atoms: &[ChangeAtom]) -> String {
    let mut digest = Sha256::new();
    digest.update(repository.base_commit.as_bytes());
    for atom in atoms {
        digest.update(atom.id.as_bytes());
        digest.update(atom.captured_bytes());
    }
    full_digest(&digest.finalize())
}

fn command_digest(command: &BuildCommand, toolchain: &BuildToolchainProvenance) -> String {
    let bytes = serde_json::to_vec(&(command, toolchain))
        .expect("serializing report-owned command provenance cannot fail");
    full_digest(&Sha256::digest(bytes))
}

fn full_digest(bytes: &[u8]) -> String {
    crate::hex_prefix(bytes, bytes.len())
}

fn run_artifact_name(identifiers: &[String]) -> String {
    let mut digest = Sha256::new();
    if identifiers.is_empty() {
        digest.update(b"baseline");
    } else {
        for identifier in identifiers {
            digest.update(identifier.len().to_le_bytes());
            digest.update(identifier.as_bytes());
        }
    }
    format!("run-{}", crate::hex_prefix(&digest.finalize(), 12))
}

fn resolve_program(program: &str) -> Result<PathBuf, AnalysisError> {
    let path = Path::new(program);
    if path.components().count() > 1 {
        return if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            Ok(std::env::current_dir()?.join(path))
        };
    }
    let search = std::env::var_os("PATH").ok_or_else(|| {
        AnalysisError::ArtifactIntegrity("PATH is absent while fingerprinting tools".to_owned())
    })?;
    std::env::split_paths(&search)
        .map(|directory| directory.join(program))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| {
            AnalysisError::ArtifactIntegrity(format!("could not resolve tool invocation {program}"))
        })
}

fn capture_tool(
    invocation: &Path,
    version_arguments: &[&str],
    current_dir: &Path,
) -> Result<ToolIdentity, AnalysisError> {
    let resolved = invocation.canonicalize()?;
    let mut request = process_request(
        invocation.as_os_str(),
        version_arguments.iter().copied(),
        current_dir,
    );
    request.output_limit = 1024 * 1024;
    let output = run_process(&request)?;
    if output.timed_out || output.exit_code != Some(0) || output.stdout_truncated {
        return Err(AnalysisError::ArtifactIntegrity(format!(
            "could not fingerprint {}",
            invocation.display()
        )));
    }
    let invocation_bytes = fs::read(invocation)?;
    let resolved_bytes = fs::read(&resolved)?;
    Ok(ToolIdentity {
        invocation_path: invocation.to_string_lossy().into_owned(),
        invocation_sha256: full_digest(&Sha256::digest(invocation_bytes)),
        resolved_path: resolved.to_string_lossy().into_owned(),
        resolved_sha256: full_digest(&Sha256::digest(resolved_bytes)),
        version: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
    })
}

fn rustup_which(tool: &str, current_dir: &Path) -> Option<PathBuf> {
    let rustup = resolve_program("rustup").ok()?;
    let request = process_request(
        rustup.as_os_str(),
        [OsString::from("which"), OsString::from(tool)],
        current_dir,
    );
    let output = run_process(&request).ok()?;
    if output.timed_out || output.exit_code != Some(0) || output.stdout_truncated {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

fn capture_toolchain(
    current_dir: &Path,
    command: &BuildCommand,
) -> Result<BuildToolchainProvenance, AnalysisError> {
    let adapter = command.adapter()?;
    let sandbox_path = resolve_program("bwrap")?;
    let mut tools = Vec::new();
    let mut support_files = Vec::new();
    match adapter {
        BuildAdapter::CargoRustc => {
            let cargo_path = resolve_program(&command.program)?;
            let rustc_path = resolve_program("rustc")?;
            tools.push(named_tool(
                "driver",
                capture_tool(&cargo_path, &["-Vv"], current_dir)?,
            ));
            tools.push(named_tool(
                "compiler",
                capture_tool(&rustc_path, &["-vV"], current_dir)?,
            ));
            if let Some(path) =
                rustup_which("cargo", current_dir).filter(|path| path != &cargo_path)
            {
                tools.push(named_tool(
                    "delegated_driver",
                    capture_tool(&path, &["-Vv"], current_dir)?,
                ));
            }
            if let Some(path) =
                rustup_which("rustc", current_dir).filter(|path| path != &rustc_path)
            {
                tools.push(named_tool(
                    "delegated_compiler",
                    capture_tool(&path, &["-vV"], current_dir)?,
                ));
            }
        }
        BuildAdapter::Clang | BuildAdapter::Gcc => {
            let driver_path = resolve_program(&command.program)?;
            tools.push(named_tool(
                "driver",
                capture_tool(&driver_path, &["--version"], current_dir)?,
            ));
        }
        BuildAdapter::TypeScript => {
            let root = typescript_adapter_root()?;
            let node_path = resolve_program("node")?;
            let compiler_path = typescript_native_compiler(&root)?;
            tools.push(named_tool(
                "driver",
                capture_tool(&node_path, &["--version"], current_dir)?,
            ));
            tools.push(named_tool(
                "compiler",
                capture_tool(&compiler_path, &["--version"], current_dir)?,
            ));
            support_files.push(support_file("adapter", &root.join("diagnostics.mjs"))?);
            support_files.push(support_file(
                "dependency_lock",
                &root.join("package-lock.json"),
            )?);
        }
    }
    tools.sort_by(|left, right| left.role.cmp(&right.role));
    support_files.sort_by(|left, right| left.role.cmp(&right.role));
    Ok(BuildToolchainProvenance {
        adapter: adapter.report_name().to_owned(),
        sandbox: BuildSandboxProvenance {
            provider: "bubblewrap".to_owned(),
            tool: capture_tool(&sandbox_path, &["--version"], current_dir)?,
            network_isolated: true,
            host_root_read_only: true,
            private_tmp: true,
        },
        tools,
        support_files,
    })
}

fn named_tool(role: &str, identity: ToolIdentity) -> NamedToolIdentity {
    NamedToolIdentity {
        role: role.to_owned(),
        identity,
    }
}

fn support_file(role: &str, path: &Path) -> Result<SupportFileIdentity, AnalysisError> {
    let path = path.canonicalize()?;
    Ok(SupportFileIdentity {
        role: role.to_owned(),
        path: path.to_string_lossy().into_owned(),
        sha256: full_digest(&Sha256::digest(fs::read(&path)?)),
    })
}

fn typescript_adapter_root() -> Result<PathBuf, AnalysisError> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/typescript-adapter")
        .canonicalize()
        .map_err(AnalysisError::Io)
}

fn typescript_native_compiler(root: &Path) -> Result<PathBuf, AnalysisError> {
    let packages = root.join("node_modules/@typescript");
    let mut candidates = fs::read_dir(&packages)
        .map_err(|error| {
            AnalysisError::ArtifactIntegrity(format!(
                "TypeScript adapter dependencies are absent at {}; run npm ci in {}: {error}",
                packages.display(),
                root.display()
            ))
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("lib/tsc"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    candidates.sort();
    match candidates.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err(AnalysisError::ArtifactIntegrity(format!(
            "TypeScript native compiler is absent; run npm ci in {}",
            root.display()
        ))),
        _ => Err(AnalysisError::ArtifactIntegrity(format!(
            "multiple TypeScript native compilers were found in {}",
            packages.display()
        ))),
    }
}

fn execution_driver(
    toolchain: &BuildToolchainProvenance,
) -> Result<(PathBuf, Vec<OsString>), AnalysisError> {
    let driver = toolchain
        .tools
        .iter()
        .find(|tool| tool.role == "driver")
        .ok_or_else(|| AnalysisError::ArtifactIntegrity("toolchain driver is absent".to_owned()))?;
    let prefix = if toolchain.adapter == "typescript" {
        let adapter = toolchain
            .support_files
            .iter()
            .find(|file| file.role == "adapter")
            .ok_or_else(|| {
                AnalysisError::ArtifactIntegrity(
                    "TypeScript adapter support file is absent".to_owned(),
                )
            })?;
        vec![OsString::from(&adapter.path)]
    } else {
        Vec::new()
    };
    Ok((PathBuf::from(&driver.identity.invocation_path), prefix))
}

fn semantic_digest(report: &BuildCausalityReport) -> Result<String, AnalysisError> {
    let mut value = serde_json::to_value(report)?;
    strip_non_semantic_fields(&mut value);
    Ok(full_digest(&Sha256::digest(serde_json::to_vec(&value)?)))
}

fn strip_non_semantic_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            for field in [
                "analysis_id",
                "artifact_path",
                "artifacts",
                "repository",
                "rendered",
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

fn verify_artifacts(
    report_path: &Path,
    report: &BuildCausalityReport,
) -> Result<(), AnalysisError> {
    let root = report_path.parent().ok_or_else(|| {
        AnalysisError::ArtifactIntegrity("report path has no parent directory".to_owned())
    })?;
    ArtifactStore::new(root).verify(&report.artifacts)?;
    Ok(())
}

/// Re-executes a retained build-causality report and verifies its semantic projection.
///
/// # Errors
///
/// Returns `AnalysisError` when retained artifacts fail integrity checks, the
/// captured working-tree input or toolchain changed, or the replayed compiler
/// observation differs.
pub fn replay_build(report_path: &Path) -> Result<ReplayResult, AnalysisError> {
    let original: BuildCausalityReport = serde_json::from_slice(&fs::read(report_path)?)?;
    verify_artifacts(report_path, &original)?;
    if semantic_digest(&original)? != original.semantic_digest {
        return Err(AnalysisError::ArtifactIntegrity(
            "report semantic digest does not match its contents".to_owned(),
        ));
    }
    let repository =
        GitRepository::discover(Path::new(&original.repository), &original.base_commit)?;
    let atoms = repository.capture_atoms()?;
    let observed_input = input_digest(&repository, &atoms);
    if observed_input != original.replay.input_digest {
        return Err(AnalysisError::ReplayInputChanged {
            expected: original.replay.input_digest.clone(),
            observed: observed_input,
        });
    }
    let observed_toolchain = capture_toolchain(&repository.root, &original.command)?;
    if observed_toolchain != original.toolchain
        || command_digest(&original.command, &observed_toolchain) != original.replay.command_digest
    {
        return Err(AnalysisError::ReplayToolchainChanged);
    }
    let replayed = explain_build(&BuildCausalityRequest {
        repository: repository.root,
        base: original.base_commit.clone(),
        diagnostic: DiagnosticSelector {
            code: original.target_diagnostic.code.clone().unwrap_or_default(),
            identity: Some(original.target_diagnostic.id.clone()),
            source_path: None,
        },
        command: original.command.clone(),
        max_evaluations: original.replay.max_evaluations,
        max_cardinality: Some(original.replay.max_cardinality),
        max_hunk_evaluations: original.replay.max_hunk_evaluations,
        max_hunk_cardinality: Some(original.replay.max_hunk_cardinality),
    })?;
    if replayed.semantic_digest != original.semantic_digest {
        return Err(AnalysisError::ReplayOutcomeChanged {
            expected: original.semantic_digest,
            observed: replayed.semantic_digest,
        });
    }
    Ok(ReplayResult {
        original_analysis_id: original.analysis_id,
        replay_analysis_id: replayed.analysis_id,
        semantic_digest: replayed.semantic_digest,
        matched: true,
    })
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    struct TestRepository {
        root: PathBuf,
    }

    impl TestRepository {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let root = std::env::temp_dir()
                .join(format!("whyvec-build-test-{}-{unique}", std::process::id()));
            fs::create_dir_all(root.join("src")).expect("create test repository");
            fs::write(
                root.join("Cargo.toml"),
                "[package]\nname = \"causality-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
            )
            .expect("write manifest");
            fs::write(
                root.join("src/api.rs"),
                "pub trait Marker {}\npub struct Item;\n\n\npub fn measure(value: i32) -> usize {\n\n    value as usize\n}\n\n\npub fn stable() -> usize { 1 }\n",
            )
            .expect("write API");
            fs::write(
                root.join("src/lib.rs"),
                "pub mod api;\npub mod consumer;\npub mod other;\npub fn run() -> usize { api::measure(7) }\n",
            )
            .expect("write caller");
            fs::write(
                root.join("src/consumer.rs"),
                "use crate::api;\npub const HANDLER: fn(i32) -> usize = api::measure;\n",
            )
            .expect("write second consumer");
            fs::write(
                root.join("src/other.rs"),
                "pub fn label() -> &'static str { \"base\" }\n",
            )
            .expect("write unrelated module");
            git_test(&root, ["init", "--quiet"]);
            git_test(&root, ["config", "user.email", "whyvec@example.invalid"]);
            git_test(&root, ["config", "user.name", "WhyVec Test"]);
            git_test(&root, ["add", "."]);
            git_test(&root, ["commit", "--quiet", "-m", "base"]);
            Self { root }
        }
    }

    impl Drop for TestRepository {
        fn drop(&mut self) {
            let _ = Command::new("git")
                .args(["worktree", "prune", "--expire", "now"])
                .current_dir(&self.root)
                .status();
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn git_test<const N: usize>(root: &Path, arguments: [&str; N]) {
        let status = Command::new("git")
            .args(arguments)
            .current_dir(root)
            .status()
            .expect("run git");
        assert!(status.success());
    }

    #[test]
    fn isolates_one_failure_inducing_file_and_confirms_removal() {
        let repository = TestRepository::new();
        fs::write(
            repository.root.join("src/api.rs"),
            "pub trait Marker {}\npub struct Item;\n\n\npub fn measure(value: &str) -> usize {\n\n    value.len()\n}\n\n\npub fn stable() -> usize { 2 }\n",
        )
        .expect("change API");
        fs::write(
            repository.root.join("src/other.rs"),
            "pub fn label() -> &'static str { \"changed\" }\n",
        )
        .expect("change unrelated module");
        fs::write(repository.root.join("notes.txt"), "untracked context\n")
            .expect("write untracked file");

        let report = explain_build(&BuildCausalityRequest {
            repository: repository.root.clone(),
            base: "HEAD".to_owned(),
            diagnostic: DiagnosticSelector {
                code: "E0308".to_owned(),
                identity: None,
                source_path: Some("src/lib.rs".to_owned()),
            },
            command: BuildCommand::cargo(["check"]),
            max_evaluations: 16,
            max_cardinality: None,
            max_hunk_evaluations: 16,
            max_hunk_cardinality: None,
        })
        .expect("causality analysis succeeds");

        assert_eq!(report.minimality, "smallest_set_found");
        assert_eq!(report.causal_sets.len(), 1);
        assert_eq!(report.causal_sets[0].sufficient_files, ["src/api.rs"]);
        assert!(report.causal_sets[0].target_removed_from_full_patch);
        assert_eq!(report.hunk_refinements.len(), 1);
        assert_eq!(report.hunk_refinements[0].hunks.len(), 3);
        assert_eq!(report.hunk_refinements[0].grouping, "rust_item");
        assert_eq!(report.hunk_refinements[0].syntax_groups.len(), 2);
        assert!(report.hunk_refinements[0].syntax_groups.iter().any(
            |group| group.member_hunks.len() == 2 && group.symbol.as_deref() == Some("measure")
        ));
        assert_eq!(report.hunk_refinements[0].causal_sets.len(), 1);
        assert_eq!(
            report.hunk_refinements[0].causal_sets[0]
                .sufficient_hunks
                .len(),
            2
        );
        assert!(report.hunk_refinements[0].causal_sets[0].target_removed_from_full_patch);
        assert!(
            report.causal_sets[0]
                .diagnostics_suppressed_with_target
                .len()
                >= 2
        );
        assert!(Path::new(&report.artifact_path).is_file());
        assert!(
            report
                .atoms
                .iter()
                .any(|atom| atom.display == "untracked notes.txt")
        );
    }

    #[test]
    fn build_adapter_rejects_non_json_diagnostic_output() {
        let command = BuildCommand::cargo(["check", "--message-format=short"]);
        assert!(matches!(
            command.adapter_arguments(),
            Err(AnalysisError::UnsupportedMessageFormat(format)) if format == "short"
        ));
    }

    #[test]
    fn build_adapter_rejects_cargo_named_wrapper_paths() {
        let command = BuildCommand {
            program: "./cargo".to_owned(),
            arguments: vec!["check".to_owned()],
        };
        assert!(matches!(
            command.adapter_arguments(),
            Err(AnalysisError::UnsupportedBuildCommand(program)) if program == "./cargo"
        ));
    }

    #[test]
    fn build_adapter_rejects_compiler_paths_and_code_loading_flags() {
        for command in [
            BuildCommand {
                program: "./gcc".to_owned(),
                arguments: vec!["-fsyntax-only".to_owned(), "source.c".to_owned()],
            },
            BuildCommand {
                program: "clang".to_owned(),
                arguments: vec!["@untrusted.rsp".to_owned()],
            },
            BuildCommand {
                program: "g++".to_owned(),
                arguments: vec!["-fplugin=untrusted.so".to_owned()],
            },
        ] {
            assert!(matches!(
                command.adapter_arguments(),
                Err(AnalysisError::UnsupportedBuildCommand(_))
            ));
        }
    }

    #[test]
    fn build_adapter_rejects_unstructured_native_diagnostics() {
        let clang = BuildCommand {
            program: "clang".to_owned(),
            arguments: vec!["-fdiagnostics-format=text".to_owned()],
        };
        assert!(matches!(
            clang.adapter_arguments(),
            Err(AnalysisError::UnsupportedDiagnosticFormat { adapter, format })
                if adapter == "clang" && format == "text"
        ));

        let gcc = BuildCommand {
            program: "gcc".to_owned(),
            arguments: vec!["-fdiagnostics-format=text".to_owned()],
        };
        assert!(matches!(
            gcc.adapter_arguments(),
            Err(AnalysisError::UnsupportedDiagnosticFormat { adapter, format })
                if adapter == "gcc" && format == "text"
        ));
    }

    #[test]
    fn finds_interacting_hunks_when_neither_hunk_fails_alone() {
        let repository = TestRepository::new();
        fs::write(
            repository.root.join("src/api.rs"),
            "pub trait Marker {}\npub struct Item;\nimpl Marker for Item {}\n\n\npub fn measure(value: i32) -> usize {\n\n    value as usize\n}\n\n\npub fn stable() -> usize { 1 }\n\n\nimpl Marker for Item {}\n",
        )
        .expect("add conflicting implementations");

        let report = explain_build(&BuildCausalityRequest {
            repository: repository.root.clone(),
            base: "HEAD".to_owned(),
            diagnostic: DiagnosticSelector {
                code: "E0119".to_owned(),
                identity: None,
                source_path: Some("src/api.rs".to_owned()),
            },
            command: BuildCommand::cargo(["check"]),
            max_evaluations: 16,
            max_cardinality: None,
            max_hunk_evaluations: 16,
            max_hunk_cardinality: None,
        })
        .expect("interacting-hunk analysis succeeds");

        assert_eq!(report.hunk_refinements.len(), 1);
        let refinement = &report.hunk_refinements[0];
        assert_eq!(refinement.hunks.len(), 2);
        assert_eq!(refinement.causal_sets.len(), 1);
        assert_eq!(refinement.causal_sets[0].sufficient_hunks.len(), 2);
        assert_eq!(refinement.minimality, "unique_minimal_in_declared_search");
        assert!(refinement.causal_sets[0].target_removed_from_full_patch);
    }

    #[test]
    fn build_adapter_accepts_cargo_json_variants() {
        let command =
            BuildCommand::cargo(["check", "--message-format", "json-diagnostic-rendered-ansi"]);
        let arguments = command.adapter_arguments().expect("JSON format accepted");
        assert_eq!(
            arguments,
            [
                OsString::from("check"),
                OsString::from("--message-format"),
                OsString::from("json-diagnostic-rendered-ansi"),
            ]
        );
    }
}
