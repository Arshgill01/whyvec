use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file: String,
    pub line: u64,
    pub column: u64,
    pub label: Option<String>,
    pub source_excerpt: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticRecord {
    pub id: String,
    pub adapter: String,
    pub code: Option<String>,
    pub level: String,
    pub message: String,
    pub target: Option<String>,
    pub primary_span: Option<SourceSpan>,
    pub rendered: Option<String>,
}

impl DiagnosticRecord {
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.level == "error"
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticSelector {
    pub code: String,
    pub identity: Option<String>,
    pub source_path: Option<String>,
}

impl DiagnosticSelector {
    #[must_use]
    pub fn matches(&self, diagnostic: &DiagnosticRecord) -> bool {
        if self
            .identity
            .as_ref()
            .is_some_and(|identity| &diagnostic.id != identity)
        {
            return false;
        }
        if diagnostic.code.as_deref() != Some(self.code.as_str()) {
            return false;
        }
        self.source_path.as_ref().is_none_or(|expected| {
            diagnostic
                .primary_span
                .as_ref()
                .is_some_and(|span| &span.file == expected)
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticSelectionError {
    NotFound {
        selector: DiagnosticSelector,
        available: Vec<DiagnosticRecord>,
    },
    Ambiguous {
        selector: DiagnosticSelector,
        matches: Vec<DiagnosticRecord>,
    },
}

impl std::fmt::Display for DiagnosticSelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound {
                selector,
                available,
            } => {
                write!(
                    formatter,
                    "diagnostic {} was not found; available errors:",
                    selector.code
                )?;
                for diagnostic in available {
                    write!(formatter, "\n  {}", diagnostic_summary(diagnostic))?;
                }
                Ok(())
            }
            Self::Ambiguous { selector, matches } => {
                write!(
                    formatter,
                    "diagnostic {} matched {} observations; rerun with one stable diagnostic id:",
                    selector.code,
                    matches.len()
                )?;
                for diagnostic in matches {
                    write!(formatter, "\n  {}", diagnostic_summary(diagnostic))?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for DiagnosticSelectionError {}

/// Selects one unique error diagnostic from a Cargo compiler-message stream.
///
/// # Errors
///
/// Returns `DiagnosticSelectionError` when the selector matches no diagnostic
/// or more than one stable diagnostic identity.
pub fn select_diagnostic(
    diagnostics: &[DiagnosticRecord],
    selector: &DiagnosticSelector,
) -> Result<DiagnosticRecord, DiagnosticSelectionError> {
    let unique_errors = deduplicate(diagnostics.iter().filter(|item| item.is_error()).cloned());
    let matches = unique_errors
        .iter()
        .filter(|diagnostic| selector.matches(diagnostic))
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err(DiagnosticSelectionError::NotFound {
            selector: selector.clone(),
            available: unique_errors,
        }),
        _ => Err(DiagnosticSelectionError::Ambiguous {
            selector: selector.clone(),
            matches,
        }),
    }
}

#[must_use]
pub fn parse_cargo_json(output: &[u8], worktree: &Path) -> Vec<DiagnosticRecord> {
    let text = String::from_utf8_lossy(output);
    let mut diagnostics = Vec::new();
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value.get("reason").and_then(Value::as_str) != Some("compiler-message") {
            continue;
        }
        let Some(message) = value.get("message") else {
            continue;
        };
        let level = message
            .get("level")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let text = message
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let code = message
            .get("code")
            .and_then(|code| code.get("code"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let target = value
            .get("target")
            .and_then(|target| target.get("name"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let primary_span = message
            .get("spans")
            .and_then(Value::as_array)
            .and_then(|spans| {
                spans
                    .iter()
                    .find(|span| span.get("is_primary") == Some(&Value::Bool(true)))
            })
            .and_then(|span| parse_span(span, worktree));
        let rendered = message
            .get("rendered")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let id = fingerprint(
            code.as_deref(),
            level,
            text,
            target.as_deref(),
            primary_span.as_ref(),
        );
        diagnostics.push(DiagnosticRecord {
            id,
            adapter: "rustc".to_owned(),
            code,
            level: level.to_owned(),
            message: normalize_whitespace(text),
            target,
            primary_span,
            rendered,
        });
    }
    deduplicate(diagnostics)
}

fn parse_span(value: &Value, worktree: &Path) -> Option<SourceSpan> {
    let file_name = value.get("file_name")?.as_str()?;
    let file = normalize_path(file_name, worktree);
    let line = value.get("line_start")?.as_u64()?;
    let column = value.get("column_start")?.as_u64()?;
    let label = value
        .get("label")
        .and_then(Value::as_str)
        .map(normalize_whitespace);
    let source_excerpt = value
        .get("text")
        .and_then(Value::as_array)
        .map(|lines| {
            lines
                .iter()
                .filter_map(|line| line.get("text").and_then(Value::as_str))
                .map(str::trim)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|text| !text.is_empty());
    Some(SourceSpan {
        file,
        line,
        column,
        label,
        source_excerpt,
    })
}

fn fingerprint(
    code: Option<&str>,
    level: &str,
    message: &str,
    target: Option<&str>,
    span: Option<&SourceSpan>,
) -> String {
    let mut digest = Sha256::new();
    for field in [
        "rustc",
        code.unwrap_or(""),
        level,
        &normalize_whitespace(message),
        target.unwrap_or(""),
        span.map_or("", |span| span.file.as_str()),
        span.and_then(|span| span.label.as_deref()).unwrap_or(""),
        span.and_then(|span| span.source_excerpt.as_deref())
            .unwrap_or(""),
    ] {
        digest.update(field.len().to_le_bytes());
        digest.update(field.as_bytes());
    }
    let bytes = digest.finalize();
    let short = crate::hex_prefix(&bytes, 10);
    format!("rustc:{}:{short}", code.unwrap_or("uncoded"))
}

fn normalize_path(file_name: &str, worktree: &Path) -> String {
    let path = Path::new(file_name);
    let relative = if path.is_absolute() {
        path.strip_prefix(worktree).unwrap_or(path)
    } else {
        path
    };
    relative.to_string_lossy().replace('\\', "/")
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn diagnostic_summary(diagnostic: &DiagnosticRecord) -> String {
    let location = diagnostic.primary_span.as_ref().map_or_else(
        || "unknown location".to_owned(),
        |span| format!("{}:{}:{}", span.file, span.line, span.column),
    );
    format!("{} at {} — {}", diagnostic.id, location, diagnostic.message)
}

fn deduplicate(diagnostics: impl IntoIterator<Item = DiagnosticRecord>) -> Vec<DiagnosticRecord> {
    let mut unique = BTreeMap::new();
    for diagnostic in diagnostics {
        unique.entry(diagnostic.id.clone()).or_insert(diagnostic);
    }
    unique.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_survives_line_number_and_worktree_changes() {
        let first = br#"{"reason":"compiler-message","target":{"name":"demo"},"message":{"code":{"code":"E0308"},"level":"error","message":"mismatched types","rendered":null,"spans":[{"file_name":"/tmp/a/src/lib.rs","line_start":7,"column_start":5,"is_primary":true,"label":"expected &str","text":[{"text":"call(42)"}]}]}}"#;
        let second = br#"{"reason":"compiler-message","target":{"name":"demo"},"message":{"code":{"code":"E0308"},"level":"error","message":"mismatched types","rendered":null,"spans":[{"file_name":"/tmp/b/src/lib.rs","line_start":19,"column_start":5,"is_primary":true,"label":"expected &str","text":[{"text":"call(42)"}]}]}}"#;
        let first = parse_cargo_json(first, Path::new("/tmp/a"));
        let second = parse_cargo_json(second, Path::new("/tmp/b"));
        assert_eq!(first[0].id, second[0].id);
        assert_ne!(first[0].primary_span, second[0].primary_span);
    }

    #[test]
    fn selector_requires_unique_error() {
        let output = br#"{"reason":"compiler-message","target":{"name":"demo"},"message":{"code":{"code":"E0308"},"level":"error","message":"first","rendered":null,"spans":[{"file_name":"src/a.rs","line_start":1,"column_start":1,"is_primary":true,"label":null,"text":[{"text":"a"}]}]}}
{"reason":"compiler-message","target":{"name":"demo"},"message":{"code":{"code":"E0308"},"level":"error","message":"second","rendered":null,"spans":[{"file_name":"src/b.rs","line_start":1,"column_start":1,"is_primary":true,"label":null,"text":[{"text":"b"}]}]}}"#;
        let diagnostics = parse_cargo_json(output, Path::new("."));
        let selector = DiagnosticSelector {
            code: "E0308".to_owned(),
            identity: None,
            source_path: None,
        };
        assert!(matches!(
            select_diagnostic(&diagnostics, &selector),
            Err(DiagnosticSelectionError::Ambiguous { .. })
        ));
    }
}
