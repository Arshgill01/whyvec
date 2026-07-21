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

/// Selects one unique error diagnostic from a normalized compiler stream.
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
            "rustc",
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

#[must_use]
pub fn parse_gcc_json(output: &[u8], worktree: &Path) -> Vec<DiagnosticRecord> {
    let Ok(values) = serde_json::from_slice::<Vec<Value>>(output) else {
        return Vec::new();
    };
    let diagnostics = values.into_iter().filter_map(|value| {
        let level = value.get("kind")?.as_str()?;
        let message = value.get("message")?.as_str()?;
        let code = value
            .get("option")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| Some(format!("gcc-{level}")));
        let primary_span = value
            .get("locations")
            .and_then(Value::as_array)
            .and_then(|locations| locations.first())
            .and_then(|location| location.get("caret"))
            .and_then(|caret| machine_span(caret, value.get("locations"), worktree));
        let id = fingerprint(
            "gcc",
            code.as_deref(),
            level,
            message,
            None,
            primary_span.as_ref(),
        );
        Some(DiagnosticRecord {
            id,
            adapter: "gcc".to_owned(),
            code,
            level: level.to_owned(),
            message: normalize_whitespace(message),
            target: None,
            primary_span,
            rendered: None,
        })
    });
    deduplicate(diagnostics)
}

#[must_use]
pub fn parse_clang_sarif(output: &[u8], worktree: &Path) -> Vec<DiagnosticRecord> {
    let text = String::from_utf8_lossy(output);
    let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text[start..=end]) else {
        return Vec::new();
    };
    let mut diagnostics = Vec::new();
    for run in value
        .get("runs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let artifacts = run.get("artifacts").and_then(Value::as_array);
        for result in run
            .get("results")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let level = result
                .get("level")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let message = result
                .get("message")
                .and_then(|message| message.get("text"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let code = result
                .get("ruleId")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| Some(format!("clang-{level}")));
            let primary_span = result
                .get("locations")
                .and_then(Value::as_array)
                .and_then(|locations| locations.first())
                .and_then(|location| location.get("physicalLocation"))
                .and_then(|location| sarif_span(location, artifacts, worktree));
            let id = fingerprint(
                "clang",
                code.as_deref(),
                level,
                message,
                None,
                primary_span.as_ref(),
            );
            diagnostics.push(DiagnosticRecord {
                id,
                adapter: "clang".to_owned(),
                code,
                level: level.to_owned(),
                message: normalize_whitespace(message),
                target: None,
                primary_span,
                rendered: None,
            });
        }
    }
    deduplicate(diagnostics)
}

#[must_use]
pub fn parse_typescript_json(output: &[u8], worktree: &Path) -> Vec<DiagnosticRecord> {
    let Ok(value) = serde_json::from_slice::<Value>(output) else {
        return Vec::new();
    };
    let diagnostics = value
        .get("diagnostics")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|diagnostic| {
            let code = diagnostic.get("code")?.as_str()?.to_owned();
            let level = diagnostic.get("category")?.as_str()?;
            let message = diagnostic.get("message")?.as_str()?;
            let primary_span = diagnostic
                .get("file")
                .and_then(Value::as_str)
                .and_then(|file| {
                    let location = diagnostic.get("location")?;
                    let line = location.get("line")?.as_u64()?;
                    let column = location.get("column")?.as_u64()?;
                    Some(SourceSpan {
                        file: normalize_path(file, worktree),
                        line,
                        column,
                        label: None,
                        source_excerpt: read_source_excerpt(file, line, worktree),
                    })
                });
            let id = fingerprint(
                "typescript",
                Some(&code),
                level,
                message,
                None,
                primary_span.as_ref(),
            );
            Some(DiagnosticRecord {
                id,
                adapter: "typescript".to_owned(),
                code: Some(code),
                level: level.to_owned(),
                message: normalize_whitespace(message),
                target: None,
                primary_span,
                rendered: None,
            })
        });
    deduplicate(diagnostics)
}

fn machine_span(caret: &Value, locations: Option<&Value>, worktree: &Path) -> Option<SourceSpan> {
    let file = caret.get("file")?.as_str()?;
    let line = caret.get("line")?.as_u64()?;
    let column = caret
        .get("column")
        .or_else(|| caret.get("display-column"))?
        .as_u64()?;
    let label = locations
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("label"))
        .and_then(Value::as_str)
        .map(normalize_whitespace);
    Some(SourceSpan {
        file: normalize_path(file, worktree),
        line,
        column,
        label,
        source_excerpt: read_source_excerpt(file, line, worktree),
    })
}

fn sarif_span(
    location: &Value,
    artifacts: Option<&Vec<Value>>,
    worktree: &Path,
) -> Option<SourceSpan> {
    let artifact = location.get("artifactLocation")?;
    let uri = artifact.get("uri").and_then(Value::as_str).or_else(|| {
        let index = usize::try_from(artifact.get("index")?.as_u64()?).ok()?;
        artifacts?.get(index)?.get("location")?.get("uri")?.as_str()
    })?;
    let file = uri.strip_prefix("file://").unwrap_or(uri);
    let region = location.get("region")?;
    let line = region.get("startLine")?.as_u64()?;
    let column = region.get("startColumn")?.as_u64()?;
    Some(SourceSpan {
        file: normalize_path(file, worktree),
        line,
        column,
        label: None,
        source_excerpt: read_source_excerpt(file, line, worktree),
    })
}

fn read_source_excerpt(file: &str, line: u64, worktree: &Path) -> Option<String> {
    let path = Path::new(file);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        worktree.join(path)
    };
    let index = usize::try_from(line.checked_sub(1)?).ok()?;
    std::fs::read_to_string(path)
        .ok()?
        .lines()
        .nth(index)
        .map(str::trim)
        .map(str::to_owned)
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
    adapter: &str,
    code: Option<&str>,
    level: &str,
    message: &str,
    target: Option<&str>,
    span: Option<&SourceSpan>,
) -> String {
    let mut digest = Sha256::new();
    for field in [
        adapter,
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
    format!("{adapter}:{}:{short}", code.unwrap_or("uncoded"))
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

    #[test]
    fn parses_gcc_native_json_identity() {
        let output = br#"[{"kind":"error","message":"invalid conversion","option":"-fpermissive","locations":[{"caret":{"file":"/tmp/repo/main.cpp","line":3,"column":7},"label":"const char*"}]}]"#;
        let diagnostics = parse_gcc_json(output, Path::new("/tmp/repo"));
        assert_eq!(diagnostics[0].adapter, "gcc");
        assert_eq!(diagnostics[0].code.as_deref(), Some("-fpermissive"));
        assert_eq!(
            diagnostics[0].primary_span.as_ref().unwrap().file,
            "main.cpp"
        );
    }

    #[test]
    fn parses_clang_sarif_after_driver_warning() {
        let output = br#"clang: warning: SARIF is unstable
{"runs":[{"artifacts":[{"location":{"uri":"file:///tmp/repo/main.cpp"}}],"results":[{"level":"error","message":{"text":"cannot initialize"},"ruleId":"3986","locations":[{"physicalLocation":{"artifactLocation":{"index":0},"region":{"startLine":2,"startColumn":4}}}]}]}]}"#;
        let diagnostics = parse_clang_sarif(output, Path::new("/tmp/repo"));
        assert_eq!(diagnostics[0].adapter, "clang");
        assert_eq!(diagnostics[0].code.as_deref(), Some("3986"));
        assert_eq!(
            diagnostics[0].primary_span.as_ref().unwrap().file,
            "main.cpp"
        );
    }

    #[test]
    fn parses_typescript_compiler_api_json() {
        let output = br#"{"adapter":"typescript","diagnostics":[{"code":"TS2322","category":"error","message":"Type string is not assignable to number","file":"/tmp/repo/src/index.ts","location":{"line":1,"column":14,"length":5}}]}"#;
        let diagnostics = parse_typescript_json(output, Path::new("/tmp/repo"));
        assert_eq!(diagnostics[0].adapter, "typescript");
        assert_eq!(diagnostics[0].code.as_deref(), Some("TS2322"));
        assert!(diagnostics[0].id.starts_with("typescript:TS2322:"));
    }
}
