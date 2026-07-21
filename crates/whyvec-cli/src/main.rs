use std::path::{Path, PathBuf};
use std::{fs, io::Write as _};

use whyvec_build::{
    BuildCausalityReport, BuildCausalityRequest, BuildCommand, DiagnosticSelector, explain_build,
    replay_build,
};
use whyvec_obligation::{ObligationRequest, derive_obligation, replay_obligation};
use whyvec_opt::{
    GccObservationReport, GccObservationRequest, OptimizationReport, OptimizationRequest,
    ParameterCandidate, explain_optimization, infer_c_source_mapping, observe_gcc_optimization,
    replay_gcc_observation, replay_optimization, resolve_compilation_command,
};

fn main() {
    match run() {
        Ok(()) => {}
        Err(error) => {
            eprintln!("whyvec: {error}");
            std::process::exit(exit_code(&error));
        }
    }
}

fn exit_code(error: &str) -> i32 {
    if error.starts_with("usage:")
        || error.contains("requires")
        || error.starts_with("unknown ")
        || error.starts_with("invalid ")
    {
        2
    } else if error.contains("compilation database")
        || error.contains("compilation entry")
        || error.contains("source mapping declined")
    {
        3
    } else if error.contains("unavailable")
        || error.contains("tool failed")
        || error.contains("toolchain")
    {
        4
    } else {
        1
    }
}

#[allow(clippy::too_many_lines)]
fn run() -> Result<(), String> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments
        .first()
        .is_some_and(|argument| argument == "--help" || argument == "-h")
    {
        println!("{}", usage());
        return Ok(());
    }
    if arguments.first().map(String::as_str) == Some("doctor") {
        return run_doctor(&arguments[1..]);
    }
    if arguments.first().map(String::as_str) == Some("analyze") {
        return run_analyze(&arguments[1..]);
    }
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

fn run_doctor(arguments: &[String]) -> Result<(), String> {
    let format = match arguments {
        [] => "human",
        [option, value] if option == "--format" && matches!(value.as_str(), "human" | "json") => {
            value
        }
        _ => return Err("usage: whyvec doctor [--format human|json]".to_owned()),
    };
    let platform_supported = cfg!(target_os = "linux") && cfg!(target_arch = "x86_64");
    let tools = [
        "clang-21",
        "opt-21",
        "llvm-config-21",
        "cmake",
        "ninja",
        "cargo",
        "rustc",
        "python3",
        "codex",
    ]
    .into_iter()
    .map(|name| (name, find_on_path(name)))
    .collect::<Vec<_>>();
    let repository = std::env::current_dir().map_err(|error| error.to_string())?;
    let helpers = ["whyvec-llvm-transform", "whyvec-llvm-loop-identity"]
        .into_iter()
        .map(|name| (name, locate_helper(name, None, &repository).ok()))
        .collect::<Vec<_>>();
    let ready = platform_supported
        && tools.iter().all(|(_, path)| path.is_some())
        && helpers.iter().all(|(_, path)| path.is_some());
    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": "1.0.0",
                "ready": ready,
                "platform": {
                    "os": std::env::consts::OS,
                    "arch": std::env::consts::ARCH,
                    "supported": platform_supported,
                },
                "tools": tools.iter().map(|(name, path)| serde_json::json!({"name": name, "path": path})).collect::<Vec<_>>(),
                "helpers": helpers.iter().map(|(name, path)| serde_json::json!({"name": name, "path": path})).collect::<Vec<_>>(),
                "exit_codes": {"success": 0, "internal": 1, "usage": 2, "decline": 3, "toolchain": 4},
            }))
            .map_err(|error| error.to_string())?
        );
    } else {
        println!("WHYVEC DOCTOR");
        println!(
            "  platform {:<24} {}",
            format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
            if platform_supported {
                "supported"
            } else {
                "unsupported"
            }
        );
        for (name, path) in tools.iter().chain(helpers.iter()) {
            let status = path
                .as_ref()
                .map_or_else(|| "missing".to_owned(), |path| path.display().to_string());
            println!("  {name:<32} {status}");
        }
        println!(
            "  status                           {}",
            if ready { "ready" } else { "not ready" }
        );
    }
    if ready {
        Ok(())
    } else {
        Err(
            "toolchain unavailable; install the pinned judge environment or complete WhyVec bundle"
                .to_owned(),
        )
    }
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| candidate.canonicalize().ok())
}

#[allow(clippy::too_many_lines)]
fn run_analyze(arguments: &[String]) -> Result<(), String> {
    let location = arguments
        .first()
        .ok_or_else(|| "analyze requires <source>:<line>".to_owned())?;
    let (source_text, line_text) = location
        .rsplit_once(':')
        .ok_or_else(|| "source location must be <path>:<line>".to_owned())?;
    let line = line_text
        .parse::<u64>()
        .map_err(|_| "source line must be a positive integer".to_owned())?;
    if line == 0 {
        return Err("source line must be positive".to_owned());
    }
    let mut repository = PathBuf::from(".");
    let mut optimizer = PathBuf::from("opt-21");
    let mut transformer = None;
    let mut identity_tool = None;
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
            "--opt" => optimizer = PathBuf::from(value(&mut index)?),
            "--transformer" => transformer = Some(PathBuf::from(value(&mut index)?)),
            "--identity-tool" => identity_tool = Some(PathBuf::from(value(&mut index)?)),
            "--format" => {
                let selected = value(&mut index)?;
                if !matches!(selected.as_str(), "human" | "json") {
                    return Err("--format must be human or json".to_owned());
                }
                format = if selected == "json" { "json" } else { "human" };
            }
            _ => return Err(format!("unknown analyze option: {option}")),
        }
        index += 1;
    }
    let repository = repository
        .canonicalize()
        .map_err(|error| format!("repository is unavailable: {error}"))?;
    let source = if Path::new(source_text).is_absolute() {
        PathBuf::from(source_text)
    } else {
        repository.join(source_text)
    };
    let source = source
        .canonicalize()
        .map_err(|error| format!("source is unavailable: {error}"))?;
    let compilation =
        resolve_compilation_command(&repository, &source).map_err(|error| error.to_string())?;
    let mapping =
        infer_c_source_mapping(&compilation, &source, line).map_err(|error| error.to_string())?;
    let transformer = locate_helper("whyvec-llvm-transform", transformer.as_deref(), &repository)?;
    let identity_tool = locate_helper(
        "whyvec-llvm-loop-identity",
        identity_tool.as_deref(),
        &repository,
    )?;
    let optimization = compilation
        .arguments
        .iter()
        .rev()
        .find_map(|argument| {
            argument
                .strip_prefix('-')
                .filter(|value| value.starts_with('O'))
        })
        .unwrap_or("O0")
        .to_owned();
    let cpu = compilation
        .arguments
        .iter()
        .rev()
        .find_map(|argument| argument.strip_prefix("-march="))
        .unwrap_or("build-command")
        .to_owned();
    let candidate_count = mapping.candidates.len();
    let report = explain_optimization(&OptimizationRequest {
        repository,
        source,
        function: mapping.function,
        line,
        candidates: mapping.candidates,
        clang: PathBuf::from(&compilation.compiler),
        optimizer,
        transformer,
        identity_tool,
        optimization,
        cpu,
        compilation: Some(compilation),
        max_evaluations: 256,
        max_cardinality: candidate_count,
    })
    .map_err(|error| error.to_string())?;
    let obligation = if report.finding.is_some() {
        Some(
            derive_obligation(&ObligationRequest {
                optimization_report: PathBuf::from(&report.artifact_path),
            })
            .map_err(|error| error.to_string())?,
        )
    } else {
        None
    };
    let agent_packet = write_agent_packet(&report, obligation.as_ref())?;
    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": "1.0.0",
                "optimization": report,
                "obligation": obligation,
                "agent_packet": agent_packet,
            }))
            .map_err(|error| error.to_string())?
        );
    } else {
        println!("WHYVEC ANALYSIS");
        println!(
            "  compilation entry: {}",
            report
                .toolchain
                .compilation
                .as_ref()
                .map_or("expert override", |command| command.database_path.as_str())
        );
        println!("  function: {}", report.replay.function);
        println!("  baseline: {}", report.monolithic_baseline.classification);
        println!();
        println!("  COUNTERFACTUAL                  OUTCOME       WIDTH  INTERLEAVE");
        println!(
            "  baseline                        {:<13} {:<6} {}",
            report.monolithic_baseline.classification,
            report
                .monolithic_baseline
                .vector_factor
                .map_or("-".to_owned(), |value| value.to_string()),
            report
                .monolithic_baseline
                .interleave_count
                .map_or("-".to_owned(), |value| value.to_string())
        );
        for experiment in &report.experiments {
            let outcome = experiment.outcome.as_ref();
            println!(
                "  {:<31} {:<13} {:<6} {}",
                experiment.assumptions.join(" + "),
                outcome.map_or("unresolved", |value| value.classification.as_str()),
                outcome
                    .and_then(|value| value.vector_factor)
                    .map_or("-".to_owned(), |value| value.to_string()),
                outcome
                    .and_then(|value| value.interleave_count)
                    .map_or("-".to_owned(), |value| value.to_string())
            );
        }
        if let Some(finding) = &report.finding {
            println!();
            println!(
                "  tested sufficient assumption: {}",
                finding.sufficient_assumptions.join(" + ")
            );
        }
        if let Some(obligation) = obligation
            .as_ref()
            .and_then(|value| value.obligation.as_ref())
        {
            println!("  candidate obligation: {}", obligation.predicate);
        }
        if let Some(decline) = &report.decline {
            println!("  declined: {} — {}", decline.code, decline.explanation);
        }
        println!("  optimization report: {}", report.artifact_path);
        if let Some(obligation) = &obligation {
            println!("  obligation report: {}", obligation.artifact_path);
        }
        println!("  agent packet: {}", agent_packet.display());
    }
    Ok(())
}

fn write_agent_packet(
    optimization: &OptimizationReport,
    obligation: Option<&whyvec_obligation::ObligationReport>,
) -> Result<PathBuf, String> {
    let repository = PathBuf::from(&optimization.repository);
    let directory = repository.join(".whyvec/agent-packets");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join(format!("{}.json", optimization.analysis_id));
    let finding = optimization.finding.as_ref();
    let derived = obligation.and_then(|report| report.obligation.as_ref());
    let packet = serde_json::json!({
        "schema_version": "1.0.0",
        "packet_id": format!("packet_{}", optimization.analysis_id.trim_start_matches("wv_")),
        "repository": optimization.repository,
        "source": optimization.source,
        "function": optimization.replay.function,
        "line": optimization.replay.line,
        "whyvec": std::env::current_exe().map_err(|error| error.to_string())?,
        "optimization": {
            "analysis_id": optimization.analysis_id,
            "semantic_digest": optimization.semantic_digest,
            "report": optimization.artifact_path,
            "baseline_observed": optimization.monolithic_baseline.classification,
            "tested_sufficient_assumptions": finding.map(|value| &value.sufficient_assumptions),
            "minimality": optimization.minimality,
        },
        "obligation": obligation.map(|report| serde_json::json!({
            "analysis_id": report.analysis_id,
            "semantic_digest": report.semantic_digest,
            "report": report.artifact_path,
            "status": if report.obligation.is_some() { "derived" } else { "declined" },
            "predicate": derived.map(|value| &value.predicate),
            "guard": derived.map(|value| &value.runtime_guard),
            "decline": report.decline,
        })),
        "repository_questions": [
            "Which public declarations and direct, indirect, generated, dynamic, or FFI callers can reach this function?",
            "Does any repository-level contract establish the full tested assumption for every caller?",
            "Can the derived condition be evaluated before any optimized-path assumption while preserving the original fallback?"
        ],
        "required_strategy_comparison": ["retain_original", "restrict", "guarded_runtime", "api_change", "refuse"],
        "required_validation": [
            "repository_native_build_and_tests",
            "fast_path_witness",
            "overlap_fallback_witness",
            "generated_defined_behavior_differential_corpus",
            "asan_ubsan",
            "structured_fast_and_fallback_compiler_records",
            "seeded_multi_size_benchmark_distribution",
            "candidate_and_validation_digest_linkage"
        ],
        "instructions": "Codex must inspect the repository and create the candidate. The packet does not authorize restrict, unconditional bound caching, or consumption of a pre-supplied candidate."
    });
    let mut bytes = serde_json::to_vec_pretty(&packet).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("cannot retain agent packet {}: {error}", path.display()))?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    Ok(path)
}

fn locate_helper(
    name: &str,
    explicit: Option<&Path>,
    repository: &Path,
) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return path
            .canonicalize()
            .map_err(|error| format!("helper {} is unavailable: {error}", path.display()));
    }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let executable_directory = executable.parent().unwrap_or(Path::new("."));
    let candidates = [
        executable_directory.join(name),
        executable_directory.join("../libexec/whyvec").join(name),
        repository.join("target/debug").join(name),
        repository.join("target/whyvec-tools").join(name),
    ];
    candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| candidate.canonicalize().ok())
        .ok_or_else(|| {
            format!(
                "required helper {name} is unavailable; run scripts/build_helpers or install the complete WhyVec bundle"
            )
        })
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
        compilation: None,
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
    "usage: whyvec analyze <source>:<line> [--repository <path>] [--format human|json]\n       whyvec doctor [--format human|json]\n       whyvec explain-build --diagnostic <code-or-id> [--at <path>] [--base <rev>] [--repository <path>] [--max-evaluations <n>] [--max-cardinality <n>] [--max-hunk-evaluations <n>] [--max-hunk-cardinality <n>] [--format human|json] -- <cargo|clang|gcc|whyvec-typescript> [arguments]\n       whyvec replay-build <report.json>\n       whyvec explain-opt <source>:<line> --function <name> --parameter <name>:<ir-index>... --transformer <path> --identity-tool <path> [--format human|json]\n       whyvec replay-opt <report.json>\n       whyvec observe-gcc-opt <source>:<line> --function <name> [--gcc <path>] [--llvm-report <report.json>] [--format human|json]\n       whyvec replay-gcc-opt <report.json>\n       whyvec derive-obligation <optimization-report.json> [--format human|json]\n       whyvec replay-obligation <report.json>\n\nexit codes: 0 success, 1 internal/evidence error, 2 usage, 3 typed decline, 4 unavailable toolchain".to_owned()
}
