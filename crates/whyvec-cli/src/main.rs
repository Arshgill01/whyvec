use std::path::PathBuf;

use whyvec_build::{
    BuildCausalityReport, BuildCausalityRequest, BuildCommand, DiagnosticSelector, explain_build,
};

fn main() {
    match run() {
        Ok(()) => {}
        Err(error) => {
            eprintln!("whyvec: {error}");
            std::process::exit(1);
        }
    }
}

#[allow(clippy::too_many_lines)]
fn run() -> Result<(), String> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.first().map(String::as_str) != Some("explain-build") {
        return Err(usage());
    }
    let separator = arguments
        .iter()
        .position(|argument| argument == "--")
        .ok_or_else(usage)?;
    let options = &arguments[1..separator];
    let command = &arguments[separator + 1..];
    if command.is_empty() {
        return Err("a Cargo command is required after --".to_owned());
    }

    let mut base = "HEAD".to_owned();
    let mut diagnostic = None;
    let mut source_path = None;
    let mut repository = PathBuf::from(".");
    let mut max_evaluations = 256_usize;
    let mut max_cardinality = None;
    let mut max_hunk_evaluations = 256_usize;
    let mut max_hunk_cardinality = None;
    let mut format = "human".to_owned();
    let mut index = 0;
    while index < options.len() {
        let option = &options[index];
        let value = |name: &str, index: &mut usize| -> Result<String, String> {
            *index += 1;
            options
                .get(*index)
                .cloned()
                .ok_or_else(|| format!("{name} requires a value"))
        };
        match option.as_str() {
            "--base" => base = value("--base", &mut index)?,
            "--diagnostic" => diagnostic = Some(value("--diagnostic", &mut index)?),
            "--at" => source_path = Some(value("--at", &mut index)?),
            "--repository" => repository = PathBuf::from(value("--repository", &mut index)?),
            "--max-evaluations" => {
                max_evaluations = parse_positive(
                    "--max-evaluations",
                    &value("--max-evaluations", &mut index)?,
                )?;
            }
            "--max-cardinality" => {
                max_cardinality = Some(parse_positive(
                    "--max-cardinality",
                    &value("--max-cardinality", &mut index)?,
                )?);
            }
            "--max-hunk-evaluations" => {
                max_hunk_evaluations = parse_positive(
                    "--max-hunk-evaluations",
                    &value("--max-hunk-evaluations", &mut index)?,
                )?;
            }
            "--max-hunk-cardinality" => {
                max_hunk_cardinality = Some(parse_positive(
                    "--max-hunk-cardinality",
                    &value("--max-hunk-cardinality", &mut index)?,
                )?);
            }
            "--format" => {
                format = value("--format", &mut index)?;
                if format != "human" && format != "json" {
                    return Err("--format must be human or json".to_owned());
                }
            }
            _ => return Err(format!("unknown option: {option}\n\n{}", usage())),
        }
        index += 1;
    }

    let diagnostic = diagnostic.ok_or_else(|| "--diagnostic is required".to_owned())?;
    let identity = diagnostic.starts_with("rustc:").then(|| diagnostic.clone());
    let diagnostic_code = if identity.is_some() {
        diagnostic
            .split(':')
            .nth(1)
            .filter(|code| !code.is_empty())
            .ok_or_else(|| "invalid rustc diagnostic identity".to_owned())?
            .to_owned()
    } else {
        diagnostic
    };
    let request = BuildCausalityRequest {
        repository,
        base,
        diagnostic: DiagnosticSelector {
            code: diagnostic_code,
            identity,
            source_path,
        },
        command: BuildCommand {
            program: command[0].clone(),
            arguments: command[1..].to_vec(),
        },
        max_evaluations,
        max_cardinality,
        max_hunk_evaluations,
        max_hunk_cardinality,
    };
    let report = explain_build(&request).map_err(|error| error.to_string())?;
    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
        );
    } else {
        print_human(&report);
    }
    Ok(())
}

fn parse_positive(name: &str, value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} must be positive"));
    }
    Ok(parsed)
}

fn print_human(report: &BuildCausalityReport) {
    println!("BUILD CAUSALITY");
    println!("  target: {}", report.target_diagnostic.id);
    println!(
        "  error:  {} {}",
        report
            .target_diagnostic
            .code
            .as_deref()
            .unwrap_or("uncoded"),
        report.target_diagnostic.message
    );
    if let Some(span) = &report.target_diagnostic.primary_span {
        println!("  at:     {}:{}:{}", span.file, span.line, span.column);
    }
    println!();
    println!("CHANGE ATOMS");
    for atom in &report.atoms {
        println!("  {}  {}", atom.id, atom.display);
    }
    println!();
    println!("COUNTERFACTUAL SEARCH");
    println!("  minimality: {}", report.minimality);
    println!("  stop:       {}", report.stop_reason);
    println!(
        "  evaluated:  {} of {} declared subsets",
        report.evaluations.len(),
        report.declared_subsets
    );
    println!();
    if report.causal_sets.is_empty() {
        println!("NO SUFFICIENT EDIT SET FOUND");
    } else {
        println!("SUFFICIENT EDIT SETS");
        for (index, causal_set) in report.causal_sets.iter().enumerate() {
            println!(
                "  {}. {}",
                index + 1,
                causal_set.sufficient_files.join(", ")
            );
            println!(
                "     removal witness: {}",
                if causal_set.target_removed_from_full_patch {
                    "target disappears"
                } else {
                    "target remains"
                }
            );
            let cascades = causal_set
                .diagnostics_suppressed_with_target
                .iter()
                .filter(|diagnostic| diagnostic.id != report.target_diagnostic.id)
                .count();
            println!("     co-suppressed diagnostics: {cascades}");
        }
    }
    for refinement in &report.hunk_refinements {
        println!();
        println!("HUNK REFINEMENT");
        println!("  minimality: {}", refinement.minimality);
        for causal_set in &refinement.causal_sets {
            println!("  sufficient syntax locations:");
            for location in &causal_set.locations {
                println!("    {location}");
            }
            println!(
                "  full-patch removal witness: {}",
                if causal_set.target_removed_from_full_patch {
                    "target disappears"
                } else {
                    "target remains"
                }
            );
        }
    }
    println!();
    println!("EVIDENCE");
    println!("  report: {}", report.artifact_path);
    println!("  claim:  tested sufficiency under the recorded Cargo build");
}

fn usage() -> String {
    "usage: whyvec explain-build --diagnostic <rustc-code> [--at <path>] [--base <rev>] [--repository <path>] [--max-evaluations <n>] [--max-cardinality <n>] [--max-hunk-evaluations <n>] [--max-hunk-cardinality <n>] [--format human|json] -- cargo check [cargo options]".to_owned()
}
