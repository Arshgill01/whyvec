//! Deterministic search over finite counterfactual intervention sets.
//!
//! This crate does not know whether an intervention is a patch atom, an LLVM
//! attribute, or a compiler flag. It owns stable enumeration, resource bounds,
//! three-valued results, and mechanically honest minimality classification.

#![forbid(unsafe_code)]

use std::fmt;

use whyvec_domain::{ExperimentVerdict, SearchMinimality};

mod artifacts;
mod process;

pub use artifacts::{ArtifactError, ArtifactReference, ArtifactStore};
pub use process::{
    ProcessError, ProcessRequest, ProcessResult, inherited_environment, process_request,
    run_process,
};

/// Stable identifier of one typed intervention supplied by an adapter or pack.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct InterventionId(String);

impl InterventionId {
    /// Creates an intervention identifier after validating its portable syntax.
    ///
    /// # Errors
    ///
    /// Returns `SearchConfigurationError::InvalidInterventionId` when the
    /// identifier is empty or contains characters outside lowercase ASCII,
    /// digits, dot, underscore, and hyphen.
    pub fn new(value: impl Into<String>) -> Result<Self, SearchConfigurationError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.as_bytes()[0].is_ascii_lowercase()
            && value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'_' | b'-')
            });
        if !valid {
            return Err(SearchConfigurationError::InvalidInterventionId(value));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Hard bounds for one deterministic sufficient-set search.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchLimits {
    pub max_cardinality: usize,
    pub max_evaluations: usize,
    pub stop_after_first_successful_cardinality: bool,
}

impl SearchLimits {
    #[must_use]
    pub const fn exhaustive(candidate_count: usize) -> Self {
        Self {
            max_cardinality: candidate_count,
            max_evaluations: usize::MAX,
            stop_after_first_successful_cardinality: false,
        }
    }
}

/// Why search ended when declared subsets remain unevaluated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchStopReason {
    DeclaredSpaceExhausted,
    FirstSuccessfulCardinalityCompleted,
    EvaluationBudgetExhausted,
}

/// One retained oracle result in stable execution order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evaluation {
    interventions: Vec<InterventionId>,
    verdict: ExperimentVerdict,
}

impl Evaluation {
    #[must_use]
    pub fn interventions(&self) -> &[InterventionId] {
        &self.interventions
    }

    #[must_use]
    pub const fn verdict(&self) -> ExperimentVerdict {
        self.verdict
    }
}

/// Complete retained result of one bounded search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchResult {
    candidates: Vec<InterventionId>,
    evaluations: Vec<Evaluation>,
    successful_sets: Vec<Vec<InterventionId>>,
    minimality: SearchMinimality,
    stop_reason: SearchStopReason,
    declared_subsets: usize,
}

impl SearchResult {
    #[must_use]
    pub fn candidates(&self) -> &[InterventionId] {
        &self.candidates
    }

    #[must_use]
    pub fn evaluations(&self) -> &[Evaluation] {
        &self.evaluations
    }

    #[must_use]
    pub fn successful_sets(&self) -> &[Vec<InterventionId>] {
        &self.successful_sets
    }

    #[must_use]
    pub const fn minimality(&self) -> SearchMinimality {
        self.minimality
    }

    #[must_use]
    pub const fn stop_reason(&self) -> SearchStopReason {
        self.stop_reason
    }

    #[must_use]
    pub const fn declared_subsets(&self) -> usize {
        self.declared_subsets
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.evaluations.len() == self.declared_subsets
    }
}

/// Invalid finite-search configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchConfigurationError {
    NoCandidates,
    DuplicateInterventionId(String),
    InvalidInterventionId(String),
    ZeroCardinality,
    ZeroEvaluationBudget,
    CardinalityExceedsCandidateCount,
    DeclaredSubsetCountOverflow,
}

impl fmt::Display for SearchConfigurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCandidates => formatter.write_str("search requires at least one candidate"),
            Self::DuplicateInterventionId(identifier) => {
                write!(formatter, "duplicate intervention identifier: {identifier}")
            }
            Self::InvalidInterventionId(identifier) => {
                write!(formatter, "invalid intervention identifier: {identifier}")
            }
            Self::ZeroCardinality => formatter.write_str("max cardinality must be positive"),
            Self::ZeroEvaluationBudget => formatter.write_str("max evaluations must be positive"),
            Self::CardinalityExceedsCandidateCount => {
                formatter.write_str("max cardinality exceeds candidate count")
            }
            Self::DeclaredSubsetCountOverflow => {
                formatter.write_str("declared subset count exceeds usize")
            }
        }
    }
}

impl std::error::Error for SearchConfigurationError {}

/// Evaluates intervention subsets in cardinality-first lexicographic order.
///
/// `oracle` must execute an isolated variant and return whether the selected
/// observation was present. Unresolved results remain first-class evidence and
/// prevent stronger minimality claims where they leave a smaller set unknown.
///
/// # Errors
///
/// Returns `SearchConfigurationError` before invoking the oracle when candidate
/// identifiers or limits are invalid.
pub fn search_sufficient_sets<F>(
    candidates: Vec<InterventionId>,
    limits: SearchLimits,
    mut oracle: F,
) -> Result<SearchResult, SearchConfigurationError>
where
    F: FnMut(&[InterventionId]) -> ExperimentVerdict,
{
    let candidates = validate_configuration(candidates, limits)?;
    let declared_subsets = declared_subset_count(candidates.len(), limits.max_cardinality)?;
    let mut evaluations = Vec::new();
    let mut successful_sets = Vec::new();
    let mut stop_reason = SearchStopReason::DeclaredSpaceExhausted;

    'cardinalities: for cardinality in 1..=limits.max_cardinality {
        let mut indexes = (0..cardinality).collect::<Vec<_>>();
        loop {
            if evaluations.len() == limits.max_evaluations {
                stop_reason = SearchStopReason::EvaluationBudgetExhausted;
                break 'cardinalities;
            }

            let intervention_set = indexes
                .iter()
                .map(|index| candidates[*index].clone())
                .collect::<Vec<_>>();
            let verdict = oracle(&intervention_set);
            if verdict == ExperimentVerdict::Observed {
                successful_sets.push(intervention_set.clone());
            }
            evaluations.push(Evaluation {
                interventions: intervention_set,
                verdict,
            });

            if !advance_combination(&mut indexes, candidates.len()) {
                break;
            }
        }

        if limits.stop_after_first_successful_cardinality
            && successful_sets
                .first()
                .is_some_and(|set| set.len() == cardinality)
        {
            if evaluations.len() < declared_subsets {
                stop_reason = SearchStopReason::FirstSuccessfulCardinalityCompleted;
            }
            break;
        }
    }

    let minimality = classify_minimality(
        &evaluations,
        &successful_sets,
        candidates.len(),
        evaluations.len() == declared_subsets,
    );
    Ok(SearchResult {
        candidates,
        evaluations,
        successful_sets,
        minimality,
        stop_reason,
        declared_subsets,
    })
}

fn validate_configuration(
    mut candidates: Vec<InterventionId>,
    limits: SearchLimits,
) -> Result<Vec<InterventionId>, SearchConfigurationError> {
    if candidates.is_empty() {
        return Err(SearchConfigurationError::NoCandidates);
    }
    if limits.max_cardinality == 0 {
        return Err(SearchConfigurationError::ZeroCardinality);
    }
    if limits.max_evaluations == 0 {
        return Err(SearchConfigurationError::ZeroEvaluationBudget);
    }
    if limits.max_cardinality > candidates.len() {
        return Err(SearchConfigurationError::CardinalityExceedsCandidateCount);
    }

    candidates.sort();
    for pair in candidates.windows(2) {
        if pair[0] == pair[1] {
            return Err(SearchConfigurationError::DuplicateInterventionId(
                pair[0].as_str().to_owned(),
            ));
        }
    }
    Ok(candidates)
}

fn declared_subset_count(
    candidate_count: usize,
    max_cardinality: usize,
) -> Result<usize, SearchConfigurationError> {
    (1..=max_cardinality).try_fold(0_usize, |total, cardinality| {
        let combinations = binomial(candidate_count, cardinality)
            .ok_or(SearchConfigurationError::DeclaredSubsetCountOverflow)?;
        total
            .checked_add(combinations)
            .ok_or(SearchConfigurationError::DeclaredSubsetCountOverflow)
    })
}

fn binomial(n: usize, k: usize) -> Option<usize> {
    let k = k.min(n - k);
    (1..=k).try_fold(1_usize, |value, index| {
        value.checked_mul(n - k + index)?.checked_div(index)
    })
}

fn advance_combination(indexes: &mut [usize], candidate_count: usize) -> bool {
    for position in (0..indexes.len()).rev() {
        let maximum = candidate_count - indexes.len() + position;
        if indexes[position] < maximum {
            indexes[position] += 1;
            for next in position + 1..indexes.len() {
                indexes[next] = indexes[next - 1] + 1;
            }
            return true;
        }
    }
    false
}

fn classify_minimality(
    evaluations: &[Evaluation],
    successful_sets: &[Vec<InterventionId>],
    candidate_count: usize,
    declared_space_exhausted: bool,
) -> SearchMinimality {
    let Some(first_success) = successful_sets.first() else {
        return SearchMinimality::NoSuccessfulSetFound;
    };
    let successful_cardinality = first_success.len();
    let smaller = evaluations
        .iter()
        .filter(|evaluation| evaluation.interventions.len() < successful_cardinality);
    if smaller
        .clone()
        .any(|evaluation| evaluation.verdict != ExperimentVerdict::NotObserved)
    {
        return SearchMinimality::SmallestSetFound;
    }
    if !declared_space_exhausted {
        return SearchMinimality::SmallestSetFound;
    }

    let observed_at_minimum = successful_sets
        .iter()
        .filter(|set| set.len() == successful_cardinality)
        .count();
    let evaluated_at_minimum = evaluations
        .iter()
        .filter(|evaluation| evaluation.interventions.len() == successful_cardinality)
        .count();
    let expected_at_minimum = binomial(candidate_count, successful_cardinality)
        .expect("validated candidate count and cardinality fit the declared search");

    if evaluated_at_minimum != expected_at_minimum {
        return SearchMinimality::SmallestSetFound;
    }
    if evaluations.iter().any(|evaluation| {
        evaluation.interventions.len() == successful_cardinality
            && matches!(evaluation.verdict, ExperimentVerdict::Unresolved(_))
    }) {
        return SearchMinimality::MinimalInDeclaredSearch;
    }
    if observed_at_minimum == 1 {
        SearchMinimality::UniqueMinimalInDeclaredSearch
    } else {
        SearchMinimality::MinimalInDeclaredSearch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use whyvec_domain::UnresolvedReason;

    fn ids(values: &[&str]) -> Vec<InterventionId> {
        values
            .iter()
            .map(|value| InterventionId::new(*value).expect("valid test id"))
            .collect()
    }

    #[test]
    fn finds_unique_minimal_singleton_in_stable_order() {
        let result = search_sufficient_sets(
            ids(&["z.change", "a.change", "m.change"]),
            SearchLimits {
                max_cardinality: 2,
                max_evaluations: 10,
                stop_after_first_successful_cardinality: true,
            },
            |set| {
                if set.iter().any(|item| item.as_str() == "m.change") {
                    ExperimentVerdict::Observed
                } else {
                    ExperimentVerdict::NotObserved
                }
            },
        )
        .expect("search succeeds");

        let order = result
            .evaluations()
            .iter()
            .map(|evaluation| evaluation.interventions()[0].as_str())
            .collect::<Vec<_>>();
        assert_eq!(order, ["a.change", "m.change", "z.change"]);
        assert_eq!(result.minimality(), SearchMinimality::SmallestSetFound);
        assert_eq!(
            result.stop_reason(),
            SearchStopReason::FirstSuccessfulCardinalityCompleted
        );
    }

    #[test]
    fn finds_interacting_pair_after_all_singletons_fail() {
        let result = search_sufficient_sets(
            ids(&["api.signature", "caller.update", "unrelated.format"]),
            SearchLimits {
                max_cardinality: 2,
                max_evaluations: 10,
                stop_after_first_successful_cardinality: true,
            },
            |set| {
                let values = set
                    .iter()
                    .map(InterventionId::as_str)
                    .collect::<BTreeSet<_>>();
                if values == BTreeSet::from(["api.signature", "caller.update"]) {
                    ExperimentVerdict::Observed
                } else {
                    ExperimentVerdict::NotObserved
                }
            },
        )
        .expect("search succeeds");

        assert_eq!(
            result.minimality(),
            SearchMinimality::UniqueMinimalInDeclaredSearch
        );
        assert_eq!(result.successful_sets()[0].len(), 2);
    }

    #[test]
    fn unresolved_smaller_set_blocks_minimal_claim() {
        let result = search_sufficient_sets(
            ids(&["a.change", "b.change"]),
            SearchLimits::exhaustive(2),
            |set| match set {
                [only] if only.as_str() == "a.change" => {
                    ExperimentVerdict::Unresolved(UnresolvedReason::InterventionInvalid)
                }
                [_, _] => ExperimentVerdict::Observed,
                _ => ExperimentVerdict::NotObserved,
            },
        )
        .expect("search succeeds");

        assert_eq!(result.minimality(), SearchMinimality::SmallestSetFound);
    }

    #[test]
    fn budget_exhaustion_is_explicit() {
        let result = search_sufficient_sets(
            ids(&["a", "b", "c"]),
            SearchLimits {
                max_cardinality: 2,
                max_evaluations: 2,
                stop_after_first_successful_cardinality: false,
            },
            |_| ExperimentVerdict::NotObserved,
        )
        .expect("search succeeds");

        assert_eq!(
            result.stop_reason(),
            SearchStopReason::EvaluationBudgetExhausted
        );
        assert!(!result.is_complete());
        assert_eq!(result.evaluations().len(), 2);
        assert_eq!(result.declared_subsets(), 6);
    }

    #[test]
    fn rejects_duplicate_candidates_before_oracle_execution() {
        let mut oracle_called = false;
        let error =
            search_sufficient_sets(ids(&["same", "same"]), SearchLimits::exhaustive(2), |_| {
                oracle_called = true;
                ExperimentVerdict::NotObserved
            })
            .expect_err("duplicates are rejected");

        assert_eq!(
            error,
            SearchConfigurationError::DuplicateInterventionId("same".to_owned())
        );
        assert!(!oracle_called);
    }
}
