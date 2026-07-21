//! Structured LLVM optimization-record parsing.

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RemarkDebugLocation {
    pub file: String,
    pub line: u64,
    pub column: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptimizationRemark {
    pub pass: String,
    pub kind: String,
    pub name: String,
    pub function: String,
    pub debug_location: RemarkDebugLocation,
    pub vectorization_outcome: String,
    pub vector_width: Option<u64>,
    pub interleave_count: Option<u64>,
    pub reason: String,
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedOptimizationOutcome {
    pub classification: String,
    pub vector_width: Option<u64>,
    pub interleave_count: Option<u64>,
    pub records: Vec<OptimizationRemark>,
}

#[derive(Debug)]
pub enum RemarkParseError {
    Malformed(String),
    Duplicate,
}

impl std::fmt::Display for RemarkParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(detail) => {
                write!(formatter, "malformed LLVM optimization record: {detail}")
            }
            Self::Duplicate => {
                formatter.write_str("duplicate LLVM optimization record for selected loop")
            }
        }
    }
}

impl std::error::Error for RemarkParseError {}

pub(crate) fn parse_optimization_outcome(
    bytes: &[u8],
    source: &Path,
    function: &str,
    line: u64,
) -> Result<ParsedOptimizationOutcome, RemarkParseError> {
    let mut records = Vec::new();
    for document in serde_yaml::Deserializer::from_slice(bytes) {
        let value = Value::deserialize(document)
            .map_err(|error| RemarkParseError::Malformed(error.to_string()))?;
        let Some(record) = parse_record(value)? else {
            continue;
        };
        if record.pass == "loop-vectorize"
            && record.function == function
            && record.debug_location.line == line
            && path_matches(source, &record.debug_location.file)
        {
            records.push(record);
        }
    }
    let mut unique = BTreeSet::new();
    for record in &records {
        let key = serde_json::to_string(record)
            .map_err(|error| RemarkParseError::Malformed(error.to_string()))?;
        if !unique.insert(key) {
            return Err(RemarkParseError::Duplicate);
        }
    }
    let passed = records
        .iter()
        .filter(|record| record.vectorization_outcome == "vectorized")
        .collect::<Vec<_>>();
    let missed = records
        .iter()
        .filter(|record| record.vectorization_outcome == "missed")
        .count();
    let (classification, vector_width, interleave_count) =
        if passed.len() > 1 || (!passed.is_empty() && missed > 0) {
            ("ambiguous", None, None)
        } else if let Some(record) = passed.first() {
            ("vectorized", record.vector_width, record.interleave_count)
        } else if missed > 0 {
            ("missed", None, None)
        } else {
            ("loop_absent", None, None)
        };
    Ok(ParsedOptimizationOutcome {
        classification: classification.to_owned(),
        vector_width,
        interleave_count,
        records,
    })
}

fn parse_record(value: Value) -> Result<Option<OptimizationRemark>, RemarkParseError> {
    let Value::Tagged(tagged) = value else {
        return Err(RemarkParseError::Malformed(
            "document is missing remark-kind tag".to_owned(),
        ));
    };
    let kind = tagged.tag.to_string().trim_start_matches('!').to_owned();
    let Value::Mapping(mapping) = tagged.value else {
        return Err(RemarkParseError::Malformed(
            "tagged document is not a mapping".to_owned(),
        ));
    };
    let pass = required_string(&mapping, "Pass")?;
    let name = required_string(&mapping, "Name")?;
    if pass != "loop-vectorize" {
        return Ok(None);
    }
    let function = required_string(&mapping, "Function")?;
    let location = required_mapping(&mapping, "DebugLoc")?;
    let debug_location = RemarkDebugLocation {
        file: required_string(location, "File")?,
        line: required_u64(location, "Line")?,
        column: required_u64(location, "Column")?,
    };
    let mut arguments = Vec::new();
    let mut vector_width = None;
    let mut interleave_count = None;
    if let Some(Value::Sequence(items)) = mapping.get(Value::String("Args".to_owned())) {
        for item in items {
            let Value::Mapping(argument) = item else {
                return Err(RemarkParseError::Malformed(
                    "Args item is not a mapping".to_owned(),
                ));
            };
            for (key, value) in argument {
                let key = scalar_text(key)?;
                let value = argument_text(value)?;
                if matches!(
                    key.as_str(),
                    "VectorizationFactor" | "VectorizationWidth" | "VectorWidth"
                ) {
                    vector_width = value.parse().ok();
                }
                if matches!(key.as_str(), "InterleaveCount" | "InterleavedCount") {
                    interleave_count = value.parse().ok();
                }
                arguments.push(format!("{key}={value}"));
            }
        }
    }
    let reason = arguments
        .iter()
        .filter_map(|argument| argument.strip_prefix("String="))
        .collect::<String>();
    let vectorization_outcome = if kind == "Passed" && name.eq_ignore_ascii_case("Vectorized") {
        "vectorized"
    } else if matches!(kind.as_str(), "Missed" | "Analysis" | "AnalysisFPCommute") {
        "missed"
    } else {
        "other"
    };
    Ok(Some(OptimizationRemark {
        pass,
        kind,
        name,
        function,
        debug_location,
        vectorization_outcome: vectorization_outcome.to_owned(),
        vector_width,
        interleave_count,
        reason,
        arguments,
    }))
}

fn required_mapping<'a>(mapping: &'a Mapping, key: &str) -> Result<&'a Mapping, RemarkParseError> {
    mapping
        .get(Value::String(key.to_owned()))
        .and_then(Value::as_mapping)
        .ok_or_else(|| RemarkParseError::Malformed(format!("missing {key} mapping")))
}

fn required_string(mapping: &Mapping, key: &str) -> Result<String, RemarkParseError> {
    mapping
        .get(Value::String(key.to_owned()))
        .map(scalar_text)
        .transpose()?
        .ok_or_else(|| RemarkParseError::Malformed(format!("missing {key}")))
}

fn required_u64(mapping: &Mapping, key: &str) -> Result<u64, RemarkParseError> {
    required_string(mapping, key)?
        .parse()
        .map_err(|_| RemarkParseError::Malformed(format!("{key} is not an unsigned integer")))
}

fn scalar_text(value: &Value) -> Result<String, RemarkParseError> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        _ => Err(RemarkParseError::Malformed(
            "expected scalar value".to_owned(),
        )),
    }
}

fn argument_text(value: &Value) -> Result<String, RemarkParseError> {
    scalar_text(value).or_else(|_| {
        serde_yaml::to_string(value)
            .map(|text| text.trim().replace('\n', " "))
            .map_err(|error| RemarkParseError::Malformed(error.to_string()))
    })
}

fn path_matches(source: &Path, recorded: &str) -> bool {
    let normalized = recorded.replace('\\', "/");
    let source = source.to_string_lossy().replace('\\', "/");
    source == normalized || source.ends_with(&format!("/{normalized}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSED: &str = "--- !Passed\nPass: loop-vectorize\nName: Vectorized\nDebugLoc: { File: src/kernel.c, Line: 7, Column: 3 }\nFunction: kernel\nArgs:\n  - String: 'vectorized loop width '\n  - VectorizationFactor: '8'\n  - InterleaveCount: '4'\n...\n";
    const MISSED: &str = "--- !Analysis\nPass: loop-vectorize\nName: UnsupportedUncountableLoop\nDebugLoc: { File: src/kernel.c, Line: 7, Column: 3 }\nFunction: kernel\nArgs:\n  - String: 'loop not vectorized'\n...\n--- !Missed\nPass: loop-vectorize\nName: MissedDetails\nDebugLoc: { File: src/kernel.c, Line: 7, Column: 3 }\nFunction: kernel\nArgs:\n  - String: 'loop not vectorized'\n...\n";

    #[test]
    fn parses_structured_vectorization_fields() {
        let outcome = parse_optimization_outcome(
            PASSED.as_bytes(),
            Path::new("/repo/src/kernel.c"),
            "kernel",
            7,
        )
        .unwrap();
        assert_eq!(outcome.classification, "vectorized");
        assert_eq!(outcome.vector_width, Some(8));
        assert_eq!(outcome.interleave_count, Some(4));
        assert_eq!(outcome.records[0].kind, "Passed");
    }

    #[test]
    fn recognizes_expected_analysis_and_missed_pair() {
        let outcome = parse_optimization_outcome(
            MISSED.as_bytes(),
            Path::new("/repo/src/kernel.c"),
            "kernel",
            7,
        )
        .unwrap();
        assert_eq!(outcome.classification, "missed");
        assert_eq!(outcome.records.len(), 2);
    }

    #[test]
    fn ignores_unrelated_loop_and_reports_missing_selected_record() {
        let outcome = parse_optimization_outcome(
            PASSED.as_bytes(),
            Path::new("/repo/src/kernel.c"),
            "kernel",
            8,
        )
        .unwrap();
        assert_eq!(outcome.classification, "loop_absent");
        assert!(outcome.records.is_empty());
    }

    #[test]
    fn rejects_malformed_and_duplicate_records() {
        assert!(matches!(
            parse_optimization_outcome(
                b"--- !Passed\nPass: loop-vectorize\n",
                Path::new("src/kernel.c"),
                "kernel",
                7
            ),
            Err(RemarkParseError::Malformed(_))
        ));
        let duplicate = format!("{PASSED}{PASSED}");
        assert!(matches!(
            parse_optimization_outcome(
                duplicate.as_bytes(),
                Path::new("src/kernel.c"),
                "kernel",
                7
            ),
            Err(RemarkParseError::Duplicate)
        ));
    }

    #[test]
    fn accepts_version_variation_field_names() {
        let varied = PASSED
            .replace("VectorizationFactor", "VectorizationWidth")
            .replace("InterleaveCount", "InterleavedCount");
        let outcome =
            parse_optimization_outcome(varied.as_bytes(), Path::new("src/kernel.c"), "kernel", 7)
                .unwrap();
        assert_eq!(
            (outcome.vector_width, outcome.interleave_count),
            (Some(8), Some(4))
        );
    }
}
