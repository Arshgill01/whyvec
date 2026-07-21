use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use whyvec_domain::{ExperimentVerdict, SearchMinimality, UnresolvedReason};
use whyvec_experiment::{
    InterventionId, SearchConfigurationError, SearchLimits, SearchStopReason,
    search_sufficient_sets,
};

use crate::diagnostics::{
    DiagnosticRecord, DiagnosticSelectionError, DiagnosticSelector, parse_cargo_json,
    select_diagnostic,
};
use crate::git::{
    ChangeAtom, ChangeAtomSummary, GitError, GitRepository, TextHunk, TextHunkSummary,
    apply_text_hunks,
};
use crate::process::{self, ProcessError};

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
        if Path::new(&self.program)
            .file_name()
            .and_then(|name| name.to_str())
            != Some("cargo")
        {
            return Err(AnalysisError::UnsupportedBuildCommand(self.program.clone()));
        }
        validate_message_format(&self.arguments)?;
        let mut arguments = self
            .arguments
            .iter()
            .map(OsString::from)
            .collect::<Vec<_>>();
        if !has_message_format(&self.arguments) {
            arguments.push(OsString::from("--message-format=json"));
        }
        Ok(arguments)
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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SearchEvaluationSummary {
    pub subset: Vec<String>,
    pub verdict: String,
    pub unresolved_reason: Option<String>,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub output_truncated: bool,
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
    pub sufficient_hunks: Vec<String>,
    pub locations: Vec<String>,
    pub removal_file_atoms: Vec<String>,
    pub removal_hunks: Vec<String>,
    pub target_removed_from_full_patch: bool,
    pub diagnostics_suppressed_with_target: Vec<DiagnosticRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HunkRefinementReport {
    pub parent_sufficient_atoms: Vec<String>,
    pub fixed_atoms: Vec<String>,
    pub hunks: Vec<TextHunkSummary>,
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
    NoChanges,
    BaselineFailed(Vec<DiagnosticRecord>),
    CandidateSucceeded,
    UnsupportedBuildCommand(String),
    UnsupportedMessageFormat(String),
    InterventionId(String),
    MissingCachedEvaluation(Vec<String>),
    RefinementDidNotReproduce(Vec<String>),
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

/// Executes a Cargo build-causality query in isolated detached worktrees.
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
    let temporary_root = std::env::temp_dir().join(format!("whyvec-{analysis_id}"));
    fs::create_dir(&temporary_root)?;
    let _temporary_guard = DirectoryCleanup(temporary_root.clone());
    let target_dir = temporary_root.join("cargo-target");
    fs::create_dir_all(&target_dir)?;

    let mut session = AnalysisSession {
        repository,
        atoms,
        command: request.command.clone(),
        target_dir,
        worktree_root: temporary_root.join("worktrees"),
        next_worktree: 0,
        cache: BTreeMap::new(),
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
    let mut report = BuildCausalityReport {
        schema_version: "2.0.0-dev".to_owned(),
        analysis_id,
        query_kind: "build_causality".to_owned(),
        adapter: "cargo_rustc".to_owned(),
        evidence_strength: "counterfactual_observation".to_owned(),
        repository: session.repository.root.to_string_lossy().into_owned(),
        base_commit: session.repository.base_commit.clone(),
        command: request.command.clone(),
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
        artifact_path: artifact_path.to_string_lossy().into_owned(),
        caveats: vec![
            "A sufficient edit set changes the selected compiler observation; it does not prove the edit is semantically wrong.".to_owned(),
            "Tracked text files are refined to zero-context Git hunks; these are executable edit regions, not syntax-tree-level semantic units.".to_owned(),
            "Cargo is forced offline, but build scripts are not yet operating-system sandboxed.".to_owned(),
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
    let serialized = serde_json::to_vec_pretty(&report)?;
    fs::write(&artifact_path, serialized)?;
    Ok(report)
}

struct AnalysisSession {
    repository: GitRepository,
    atoms: Vec<ChangeAtom>,
    command: BuildCommand,
    target_dir: PathBuf,
    worktree_root: PathBuf,
    next_worktree: usize,
    cache: BTreeMap<String, BuildRunSummary>,
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
        self.cache.insert(key, summary.clone());
        Ok(summary)
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

        let mut process_request = process::request(
            &self.command.program,
            self.command.adapter_arguments()?,
            worktree,
        );
        process_request.timeout = BUILD_TIMEOUT;
        process_request.output_limit = BUILD_OUTPUT_LIMIT;
        process_request.environment.extend([
            (
                OsString::from("CARGO_TARGET_DIR"),
                self.target_dir.as_os_str().to_os_string(),
            ),
            (OsString::from("CARGO_NET_OFFLINE"), OsString::from("true")),
            (OsString::from("CARGO_TERM_COLOR"), OsString::from("never")),
        ]);
        let result = process::run(&process_request)?;
        let diagnostics = parse_cargo_json(&result.stdout, worktree);
        Ok(BuildRunSummary {
            subset: identifiers.to_vec(),
            exit_code: result.exit_code,
            timed_out: result.timed_out,
            output_truncated: result.stdout_truncated || result.stderr_truncated,
            diagnostics,
        })
    }
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
    let candidates = hunks
        .iter()
        .map(|hunk| {
            InterventionId::new(hunk.summary.id.clone())
                .map_err(|_| AnalysisError::InterventionId(hunk.summary.id.clone()))
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
    let hunk_map = hunks
        .iter()
        .map(|hunk| (hunk.summary.id.clone(), hunk.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut oracle_failure = None;
    let search = search_sufficient_sets(candidates, limits, |subset| {
        let selected = subset
            .iter()
            .filter_map(|id| hunk_map.get(id.as_str()).cloned())
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
        let sufficient_ids = sufficient
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let complement = hunks
            .iter()
            .filter(|hunk| !sufficient_ids.contains(&hunk.summary.id))
            .cloned()
            .collect::<Vec<_>>();
        let removal = session.evaluate_variant(&outside_parent, &complement)?;
        let removal_ids = removal
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<BTreeSet<_>>();
        causal_sets.push(HunkCausalSetReport {
            sufficient_hunks: sufficient_ids.iter().cloned().collect(),
            locations: sufficient_ids
                .iter()
                .filter_map(|id| hunk_map.get(id))
                .map(|hunk| format!("{}:{}", hunk.summary.file, hunk.summary.new_start))
                .collect(),
            removal_file_atoms: outside_parent.clone(),
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
            cache_ids.extend(subset.iter().cloned());
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
                "pub trait Marker {}\npub struct Item;\n\n\npub fn measure(value: i32) -> usize { value as usize }\n\n\npub fn stable() -> usize { 1 }\n",
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
            "pub trait Marker {}\npub struct Item;\n\n\npub fn measure(value: &str) -> usize { value.len() }\n\n\npub fn stable() -> usize { 2 }\n",
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

        assert_eq!(report.minimality, "unique_minimal_in_declared_search");
        assert_eq!(report.causal_sets.len(), 1);
        assert_eq!(report.causal_sets[0].sufficient_files, ["src/api.rs"]);
        assert!(report.causal_sets[0].target_removed_from_full_patch);
        assert_eq!(report.hunk_refinements.len(), 1);
        assert_eq!(report.hunk_refinements[0].hunks.len(), 2);
        assert_eq!(report.hunk_refinements[0].causal_sets.len(), 1);
        assert_eq!(
            report.hunk_refinements[0].causal_sets[0]
                .sufficient_hunks
                .len(),
            1
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
    fn finds_interacting_hunks_when_neither_hunk_fails_alone() {
        let repository = TestRepository::new();
        fs::write(
            repository.root.join("src/api.rs"),
            "pub trait Marker {}\npub struct Item;\nimpl Marker for Item {}\n\n\npub fn measure(value: i32) -> usize { value as usize }\n\n\npub fn stable() -> usize { 1 }\n\n\nimpl Marker for Item {}\n",
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
