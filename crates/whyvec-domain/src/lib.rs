//! Stable domain concepts shared by `WhyVec`'s deterministic components.
//!
//! This crate deliberately has no compiler, filesystem, process, CLI, or model
//! dependencies. It defines the distinctions that other layers must preserve.

#![forbid(unsafe_code)]

use std::fmt;

/// Strength of evidence supporting a statement in a `WhyVec` report.
///
/// Ordering is intentional: a consumer may verify that a renderer or workflow
/// did not silently upgrade a claim beyond its supporting evidence.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum EvidenceStrength {
    CompilerMessage,
    CompilerRecord,
    CounterfactualObservation,
    DerivedObligation,
    RepositorySupportedContract,
    RuntimeEnforcedContract,
    FormallyVerifiedProperty,
}

/// Lifecycle of one immutable analysis record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AnalysisState {
    Created,
    Running,
    Complete,
    Declined,
    Interrupted,
    Failed,
}

impl AnalysisState {
    /// Returns whether the analysis may transition directly to `next`.
    #[must_use]
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Created, Self::Running | Self::Declined | Self::Failed)
                | (
                    Self::Running,
                    Self::Complete | Self::Declined | Self::Interrupted | Self::Failed
                )
        )
    }

    /// Returns whether no further state transition is valid.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Complete | Self::Declined | Self::Interrupted | Self::Failed
        )
    }
}

/// A concrete compiler question supported by the experiment engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum QueryKind {
    /// Determine which working-tree interventions produce a diagnostic.
    BuildCausality,
    /// Determine which compiler assumptions change an optimization decision.
    OptimizationCausality,
    /// Determine which recorded input makes compiler outcomes diverge.
    CompilerDivergence,
}

/// The compiler-owned observation tracked across counterfactual variants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ObservationKind {
    Diagnostic,
    OptimizationDecision,
    BuildStatus,
    CodeGenerationProperty,
}

/// Result of asking whether the same observation exists in one variant.
///
/// This is deliberately three-valued. A variant that cannot be compiled or
/// whose subject cannot be matched provides no evidence for or against the
/// intervention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExperimentVerdict {
    Observed,
    NotObserved,
    Unresolved(UnresolvedReason),
}

/// Why a counterfactual variant cannot answer the selected compiler question.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum UnresolvedReason {
    SubjectAbsent,
    SubjectAmbiguous,
    InterventionInvalid,
    CompileFailedForDifferentReason,
    ToolFailed,
    TimedOut,
    PolicyDenied,
    NonDeterministic,
}

/// Fidelity between the production frontend pipeline and an experiment run.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PipelineFidelity {
    /// The intervention is evaluated in a surrogate pipeline with known gaps.
    Surrogate,
    /// Independent evidence confirms the replay is equivalent for the query.
    EquivalentConfirmed,
    /// The exact recorded frontend and optimizer pipeline is replayed.
    Exact,
}

impl PipelineFidelity {
    /// Whether compiler evidence from this pipeline may enter source-action
    /// evaluation. Repository contracts and validation are still required.
    #[must_use]
    pub const fn permits_source_action_evaluation(self) -> bool {
        matches!(self, Self::EquivalentConfirmed | Self::Exact)
    }
}

/// Stable adapter family that owns command capture and observation identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AdapterKind {
    Clang,
    Rustc,
    TypeScript,
    Gcc,
}

/// Semantic level at which one experiment changes its input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum InterventionKind {
    PatchAtom,
    CompilerAssumption,
    CompilerFlag,
    ToolchainSelection,
}

/// The strongest minimality statement supported by a completed search.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SearchMinimality {
    NoSuccessfulSetFound,
    SmallestSetFound,
    MinimalInDeclaredSearch,
    UniqueMinimalInDeclaredSearch,
}

/// Why an otherwise valid analysis stopped without a finding or repair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decline {
    code: String,
    stage: DeclineStage,
    explanation: String,
    required_evidence: Vec<String>,
}

impl Decline {
    /// Creates a decline only when its stable code and explanation are present.
    ///
    /// # Errors
    ///
    /// Returns `DeclineValidationError` for an empty or malformed code or an
    /// empty explanation.
    pub fn new(
        code: impl Into<String>,
        stage: DeclineStage,
        explanation: impl Into<String>,
        required_evidence: Vec<String>,
    ) -> Result<Self, DeclineValidationError> {
        let code = code.into();
        let explanation = explanation.into();

        if !is_valid_decline_code(&code) {
            return Err(DeclineValidationError::InvalidCode);
        }
        if explanation.trim().is_empty() {
            return Err(DeclineValidationError::EmptyExplanation);
        }

        Ok(Self {
            code,
            stage,
            explanation,
            required_evidence,
        })
    }

    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    #[must_use]
    pub const fn stage(&self) -> DeclineStage {
        self.stage
    }

    #[must_use]
    pub fn explanation(&self) -> &str {
        &self.explanation
    }

    #[must_use]
    pub fn required_evidence(&self) -> &[String] {
        &self.required_evidence
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DeclineStage {
    Policy,
    AdapterResolution,
    CommandResolution,
    Baseline,
    ObservationIdentity,
    Intervention,
    LoopIdentity,
    CounterfactualSearch,
    ObligationDerivation,
    RepositoryContract,
    Validation,
    Benchmark,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclineValidationError {
    InvalidCode,
    EmptyExplanation,
}

impl fmt::Display for DeclineValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCode => {
                formatter.write_str("decline code must contain lowercase dot-separated identifiers")
            }
            Self::EmptyExplanation => formatter.write_str("decline explanation must not be empty"),
        }
    }
}

impl std::error::Error for DeclineValidationError {}

fn is_valid_decline_code(code: &str) -> bool {
    let mut segments = code.split('.');
    let valid_segment = |segment: &str| {
        !segment.is_empty()
            && segment
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            && segment.as_bytes()[0].is_ascii_lowercase()
    };

    let Some(first) = segments.next() else {
        return false;
    };
    valid_segment(first) && segments.clone().next().is_some() && segments.all(valid_segment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_strength_never_orders_observation_above_contract() {
        assert!(EvidenceStrength::CompilerRecord < EvidenceStrength::CounterfactualObservation);
        assert!(
            EvidenceStrength::CounterfactualObservation
                < EvidenceStrength::RepositorySupportedContract
        );
        assert!(
            EvidenceStrength::RepositorySupportedContract
                < EvidenceStrength::RuntimeEnforcedContract
        );
    }

    #[test]
    fn terminal_states_cannot_be_reopened() {
        for state in [
            AnalysisState::Complete,
            AnalysisState::Declined,
            AnalysisState::Interrupted,
            AnalysisState::Failed,
        ] {
            assert!(state.is_terminal());
            assert!(!state.can_transition_to(AnalysisState::Running));
        }
    }

    #[test]
    fn running_analysis_has_every_deliberate_terminal_path() {
        assert!(AnalysisState::Running.can_transition_to(AnalysisState::Complete));
        assert!(AnalysisState::Running.can_transition_to(AnalysisState::Declined));
        assert!(AnalysisState::Running.can_transition_to(AnalysisState::Interrupted));
        assert!(AnalysisState::Running.can_transition_to(AnalysisState::Failed));
    }

    #[test]
    fn unresolved_experiment_is_not_negative_evidence() {
        assert_ne!(
            ExperimentVerdict::Unresolved(UnresolvedReason::SubjectAmbiguous),
            ExperimentVerdict::NotObserved
        );
    }

    #[test]
    fn surrogate_pipeline_cannot_authorize_source_action_evaluation() {
        assert!(!PipelineFidelity::Surrogate.permits_source_action_evaluation());
        assert!(PipelineFidelity::EquivalentConfirmed.permits_source_action_evaluation());
        assert!(PipelineFidelity::Exact.permits_source_action_evaluation());
    }

    #[test]
    fn decline_requires_stable_code_and_explanation() {
        let decline = Decline::new(
            "obligation.volatile_bound",
            DeclineStage::ObligationDerivation,
            "The loop observes a volatile bound on every condition check.",
            vec!["domain-specific volatile semantics".to_owned()],
        )
        .expect("valid decline");

        assert_eq!(decline.code(), "obligation.volatile_bound");
        assert_eq!(decline.required_evidence().len(), 1);
        assert_eq!(
            Decline::new("Bad Code", DeclineStage::Policy, "denied", vec![]),
            Err(DeclineValidationError::InvalidCode)
        );
        assert_eq!(
            Decline::new("policy.denied", DeclineStage::Policy, "  ", vec![]),
            Err(DeclineValidationError::EmptyExplanation)
        );
    }
}
