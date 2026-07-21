//! Build-causality queries over isolated compiler runs.

#![forbid(unsafe_code)]

use std::fmt::Write as _;

mod analyzer;
mod diagnostics;
mod git;

pub use analyzer::{
    AnalysisError, BuildCausalityReport, BuildCausalityRequest, BuildCommand, BuildRunSummary,
    BuildToolchainProvenance, CausalSetReport, ReplayResult, ReplaySpecification,
    SearchEvaluationSummary, ToolIdentity, explain_build, replay_build,
};
pub use diagnostics::{
    DiagnosticRecord, DiagnosticSelectionError, DiagnosticSelector, SourceSpan, parse_cargo_json,
    select_diagnostic,
};
pub use git::ChangeAtomSummary;
pub use whyvec_experiment::ArtifactReference;

fn hex_prefix(bytes: &[u8], length: usize) -> String {
    bytes[..length]
        .iter()
        .fold(String::with_capacity(length * 2), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to String cannot fail");
            output
        })
}
