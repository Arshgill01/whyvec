use std::path::PathBuf;

use whyvec_build::{
    BuildCausalityReport, BuildCausalityRequest, BuildCommand, DiagnosticSelector, explain_build,
    replay_build,
};
use whyvec_obligation::{ObligationRequest, derive_obligation, replay_obligation};
use whyvec_opt::{
    GccObservationReport, GccObservationRequest, OptimizationReport, OptimizationRequest,
    ParameterCandidate, explain_optimization, observe_gcc_optimization, replay_gcc_observation,
    replay_optimization,
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
    if arguments.first().map(String::as_str) == Some("replay-build") {
        return run_replay(&arguments[1..]);
    }
    if arguments.first().map(String::as_str) == Some("replay-opt") {
        return run_opt_replay(&arguments[1..]);
    }
    if arguments.first().map(String::as_str) == Some("replay-gcc-opt") {
        return run_gcc_replay(&arguments[1..]);
    }
    if arguments.first().map(String::as_str) == Some("replay-obligation") {
        return run_obligation_replay(&arguments[1..]);
    }
    if arguments.first().map(String::as_str) == Some("derive-obligation") {
        return run_derive_obligation(&arguments[1..]);
    }
    if arguments.first().map(String::as_str) == Some("observe-gcc-opt") {
        return run_observe_gcc(&arguments[1..]);
    }
    if arguments.first().map(String::as_str) == Some("explain-opt") {
        return run_explain_opt(&arguments[1..]);
    }
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
        return Err("a supported build command is required after --".to_owned());
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
    let identity = ["rustc:", "clang:", "gcc:", "typescript:"]
        .iter()
        .any(|prefix| diagnostic.starts_with(prefix))
        .then(|| diagnostic.clone());
    let diagnostic_code = if identity.is_some() {
        diagnostic
            .split(':')
            .nth(1)
            .filter(|code| !code.is_empty())
            .ok_or_else(|| "invalid diagnostic identity".to_owned())?
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

#[allow(clippy::too_many_lines)]
fn run_explain_opt(arguments: &[String]) -> Result<(), String> {
    let location = arguments
        .first()
        .ok_or_else(|| "explain-opt requires <source>:<line>".to_owned())?;
    let (source, line) = location
        .rsplit_once(':')
        .ok_or_else(|| "source location must be <path>:<line>".to_owned())?;
    let line = line
        .parse::<u64>()
        .map_err(|_| "source line must be a positive integer".to_owned())?;
    if line == 0 {
        return Err("source line must be positive".to_owned());
    }
    let mut repository = PathBuf::from(".");
    let mut function = None;
    let mut candidates = Vec::new();
    let mut clang = PathBuf::from("clang-21");
    let mut optimizer = PathBuf::from("opt-21");
    let mut transformer = None;
    let mut identity_tool = None;
    let mut cpu = "x86-64-v3".to_owned();
    let mut max_evaluations = 256;
    let mut max_cardinality = None;
    let mut format = "human";
    let mut index = 1;
    while index < arguments.len() {
        let option = &arguments[index];
        let value = |index: &mut usize| -> Result<String, String> {
            *index += 1;
            arguments
                .get(*index)
                .cloned()
                .ok_or_else(|| format!("{option} requires a value"))
        };
        match option.as_str() {
            "--repository" => repository = PathBuf::from(value(&mut index)?),
            "--function" => function = Some(value(&mut index)?),
            "--parameter" => {
                let parameter = value(&mut index)?;
                let (name, raw_index) = parameter
                    .split_once(':')
                    .ok_or_else(|| "--parameter must be <source-name>:<ir-index>".to_owned())?;
                candidates.push(ParameterCandidate {
                    source_name: name.to_owned(),
                    ir_index: raw_index
                        .parse()
                        .map_err(|_| "IR parameter index must be non-negative".to_owned())?,
                });
            }
            "--clang" => clang = PathBuf::from(value(&mut index)?),
            "--opt" => optimizer = PathBuf::from(value(&mut index)?),
            "--transformer" => transformer = Some(PathBuf::from(value(&mut index)?)),
            "--identity-tool" => identity_tool = Some(PathBuf::from(value(&mut index)?)),
            "--cpu" => cpu = value(&mut index)?,
            "--max-evaluations" => {
                max_evaluations = parse_positive("--max-evaluations", &value(&mut index)?)?;
            }
            "--max-cardinality" => {
                max_cardinality = Some(parse_positive("--max-cardinality", &value(&mut index)?)?);
            }
            "--format" => {
                let selected = value(&mut index)?;
                if selected != "human" && selected != "json" {
                    return Err("--format must be human or json".to_owned());
                }
                format = if selected == "json" { "json" } else { "human" };
            }
            _ => return Err(format!("unknown explain-opt option: {option}")),
        }
        index += 1;
    }
    let function = function.ok_or_else(|| "--function is required".to_owned())?;
    let transformer = transformer.ok_or_else(|| "--transformer is required".to_owned())?;
    let identity_tool = identity_tool.ok_or_else(|| "--identity-tool is required".to_owned())?;
    let cardinality = max_cardinality.unwrap_or(candidates.len());
    let report = explain_optimization(&OptimizationRequest {
        repository,
        source: PathBuf::from(source),
        function,
        line,
        candidates,
        clang,
        optimizer,
        transformer,
        identity_tool,
        optimization: "O3".to_owned(),
        cpu,
        max_evaluations,
        max_cardinality: cardinality,
    })
    .map_err(|error| error.to_string())?;
    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
        );
    } else {
        print_opt_human(&report);
    }
    Ok(())
}

fn print_opt_human(report: &OptimizationReport) {
    println!("OPTIMIZATION CAUSALITY");
    println!("  baseline: {}", report.monolithic_baseline.classification);
    println!("  pipeline fidelity: {}", report.pipeline_fidelity);
    if let Some(subject) = &report.subject {
        println!("  loop fingerprint: {}", subject.structural_fingerprint);
    } else {
        println!("  loop fingerprint: unavailable");
    }
    if let Some(finding) = &report.finding {
        println!(
            "  smallest sufficient set found: {}",
            finding.sufficient_assumptions.join(", ")
        );
        println!("  {}", finding.summary);
    }
    if let Some(decline) = &report.decline {
        println!("  declined: {} — {}", decline.code, decline.explanation);
    }
    println!("  report: {}", report.artifact_path);
}

fn run_observe_gcc(arguments: &[String]) -> Result<(), String> {
    let location = arguments
        .first()
        .ok_or_else(|| "observe-gcc-opt requires <source>:<line>".to_owned())?;
    let (source, line) = location
        .rsplit_once(':')
        .ok_or_else(|| "source location must be <path>:<line>".to_owned())?;
    let line = line
        .parse::<u64>()
        .map_err(|_| "source line must be a positive integer".to_owned())?;
    if line == 0 {
        return Err("source line must be positive".to_owned());
    }
    let mut repository = PathBuf::from(".");
    let mut function = None;
    let mut gcc = PathBuf::from("gcc");
    let mut gzip = PathBuf::from("gzip");
    let mut cpu = "x86-64-v3".to_owned();
    let mut llvm_report = None;
    let mut format = "human";
    let mut index = 1;
    while index < arguments.len() {
        let option = &arguments[index];
        let value = |index: &mut usize| -> Result<String, String> {
            *index += 1;
            arguments
                .get(*index)
                .cloned()
                .ok_or_else(|| format!("{option} requires a value"))
        };
        match option.as_str() {
            "--repository" => repository = PathBuf::from(value(&mut index)?),
            "--function" => function = Some(value(&mut index)?),
            "--gcc" => gcc = PathBuf::from(value(&mut index)?),
            "--gzip" => gzip = PathBuf::from(value(&mut index)?),
            "--cpu" => cpu = value(&mut index)?,
            "--llvm-report" => llvm_report = Some(PathBuf::from(value(&mut index)?)),
            "--format" => {
                let selected = value(&mut index)?;
                if selected != "human" && selected != "json" {
                    return Err("--format must be human or json".to_owned());
                }
                format = if selected == "json" { "json" } else { "human" };
            }
            _ => return Err(format!("unknown observe-gcc-opt option: {option}")),
        }
        index += 1;
    }
    let report = observe_gcc_optimization(&GccObservationRequest {
        repository,
        source: PathBuf::from(source),
        function: function.ok_or_else(|| "--function is required".to_owned())?,
        line,
        gcc,
        gzip,
        optimization: "O3".to_owned(),
        cpu,
        llvm_report,
    })
    .map_err(|error| error.to_string())?;
    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
        );
    } else {
        print_gcc_human(&report);
    }
    Ok(())
}

fn print_gcc_human(report: &GccObservationReport) {
    println!("GCC OPTIMIZATION OBSERVATION");
    println!("  outcome: {}", report.outcome.classification);
    println!(
        "  selected records: {}",
        report.outcome.selected_remarks.len()
    );
    if let Some(comparison) = &report.comparison {
        println!("  LLVM comparison: {}", comparison.relation);
        println!("  {}", comparison.explanation);
    }
    println!("  report: {}", report.artifact_path);
}

fn run_replay(arguments: &[String]) -> Result<(), String> {
    let [report] = arguments else {
        return Err("usage: whyvec replay-build <report.json>".to_owned());
    };
    let result = replay_build(&PathBuf::from(report)).map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn run_opt_replay(arguments: &[String]) -> Result<(), String> {
    let [report] = arguments else {
        return Err("usage: whyvec replay-opt <report.json>".to_owned());
    };
    let result = replay_optimization(&PathBuf::from(report)).map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn run_gcc_replay(arguments: &[String]) -> Result<(), String> {
    let [report] = arguments else {
        return Err("usage: whyvec replay-gcc-opt <report.json>".to_owned());
    };
    let result =
        replay_gcc_observation(&PathBuf::from(report)).map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn run_derive_obligation(arguments: &[String]) -> Result<(), String> {
    let [report, rest @ ..] = arguments else {
        return Err(
            "usage: whyvec derive-obligation <optimization-report.json> [--format human|json]"
                .to_owned(),
        );
    };
    let format = match rest {
        [] => "human",
        [option, value] if option == "--format" && matches!(value.as_str(), "human" | "json") => {
            value
        }
        _ => return Err("--format must be human or json".to_owned()),
    };
    let result = derive_obligation(&ObligationRequest {
        optimization_report: PathBuf::from(report),
    })
    .map_err(|error| error.to_string())?;
    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|error| error.to_string())?
        );
    } else {
        println!("SOURCE OBLIGATION");
        if let Some(obligation) = &result.obligation {
            println!("  status: derived candidate obligation");
            println!("  family: {}", obligation.family);
            println!("  predicate: {}", obligation.predicate);
            println!("  source action: repository evidence required");
        } else if let Some(decline) = &result.decline {
            println!("  declined: {} — {}", decline.code, decline.explanation);
        }
        println!("  report: {}", result.artifact_path);
    }
    Ok(())
}

fn run_obligation_replay(arguments: &[String]) -> Result<(), String> {
    let [report] = arguments else {
        return Err("usage: whyvec replay-obligation <report.json>".to_owned());
    };
    let result = replay_obligation(&PathBuf::from(report)).map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|error| error.to_string())?
    );
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
    println!(
        "  claim:  tested sufficiency under the recorded {} build",
        report.adapter
    );
}

fn usage() -> String {
    "usage: whyvec explain-build --diagnostic <code-or-id> [--at <path>] [--base <rev>] [--repository <path>] [--max-evaluations <n>] [--max-cardinality <n>] [--max-hunk-evaluations <n>] [--max-hunk-cardinality <n>] [--format human|json] -- <cargo|clang|gcc|whyvec-typescript> [arguments]\n       whyvec replay-build <report.json>\n       whyvec explain-opt <source>:<line> --function <name> --parameter <name>:<ir-index>... --transformer <path> --identity-tool <path> [--format human|json]\n       whyvec replay-opt <report.json>\n       whyvec observe-gcc-opt <source>:<line> --function <name> [--gcc <path>] [--llvm-report <report.json>] [--format human|json]\n       whyvec replay-gcc-opt <report.json>\n       whyvec derive-obligation <optimization-report.json> [--format human|json]\n       whyvec replay-obligation <report.json>".to_owned()
}
