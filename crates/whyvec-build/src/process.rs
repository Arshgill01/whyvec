use std::ffi::{OsStr, OsString};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Clone, Debug)]
pub(crate) struct ProcessRequest {
    pub program: OsString,
    pub arguments: Vec<OsString>,
    pub current_dir: PathBuf,
    pub environment: Vec<(OsString, OsString)>,
    pub clear_environment: bool,
    pub stdin: Option<Vec<u8>>,
    pub timeout: Duration,
    pub output_limit: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct ProcessResult {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

#[derive(Debug)]
pub enum ProcessError {
    Spawn {
        program: OsString,
        source: io::Error,
    },
    Stdin(io::Error),
    Wait(io::Error),
    ReaderThreadPanicked,
    Output(io::Error),
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn { program, source } => {
                write!(
                    formatter,
                    "failed to spawn {}: {source}",
                    program.to_string_lossy()
                )
            }
            Self::Stdin(source) => write!(formatter, "failed to write process stdin: {source}"),
            Self::Wait(source) => write!(formatter, "failed while waiting for process: {source}"),
            Self::ReaderThreadPanicked => formatter.write_str("output reader thread panicked"),
            Self::Output(source) => write!(formatter, "failed to read process output: {source}"),
        }
    }
}

impl std::error::Error for ProcessError {}

pub(crate) fn run(request: &ProcessRequest) -> Result<ProcessResult, ProcessError> {
    let mut command = Command::new(&request.program);
    command
        .args(&request.arguments)
        .current_dir(&request.current_dir)
        .stdin(if request.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if request.clear_environment {
        command.env_clear();
    }
    command.envs(request.environment.iter().cloned());
    configure_process_group(&mut command);

    let mut child = command.spawn().map_err(|source| ProcessError::Spawn {
        program: request.program.clone(),
        source,
    })?;

    let stdout = child.stdout.take().expect("piped stdout exists");
    let stderr = child.stderr.take().expect("piped stderr exists");
    let output_limit = request.output_limit;
    let stdout_reader = thread::spawn(move || read_bounded(stdout, output_limit));
    let stderr_reader = thread::spawn(move || read_bounded(stderr, output_limit));

    if let Some(bytes) = &request.stdin {
        let write_result = child
            .stdin
            .take()
            .expect("piped stdin exists")
            .write_all(bytes);
        if let Err(source) = write_result {
            terminate_process_tree(&mut child);
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(ProcessError::Stdin(source));
        }
    }

    let started = Instant::now();
    let (status, timed_out) = loop {
        if let Some(status) = child.try_wait().map_err(ProcessError::Wait)? {
            break (status, false);
        }
        if started.elapsed() >= request.timeout {
            terminate_process_tree(&mut child);
            let status = child.wait().map_err(ProcessError::Wait)?;
            break (status, true);
        }
        thread::sleep(POLL_INTERVAL);
    };

    let (stdout, stdout_truncated) = stdout_reader
        .join()
        .map_err(|_| ProcessError::ReaderThreadPanicked)?
        .map_err(ProcessError::Output)?;
    let (stderr, stderr_truncated) = stderr_reader
        .join()
        .map_err(|_| ProcessError::ReaderThreadPanicked)?
        .map_err(ProcessError::Output)?;

    Ok(ProcessResult {
        exit_code: status.code(),
        stdout,
        stderr,
        timed_out,
        stdout_truncated,
        stderr_truncated,
    })
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt as _;
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
fn terminate_process_tree(child: &mut Child) {
    let process_group = format!("-{}", child.id());
    let _ = Command::new("kill")
        .args(["-KILL", "--", process_group.as_str()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(unix))]
fn terminate_process_tree(child: &mut Child) {
    let _ = child.kill();
}

fn read_bounded(mut reader: impl Read, limit: usize) -> io::Result<(Vec<u8>, bool)> {
    let mut retained = Vec::with_capacity(limit.min(64 * 1024));
    let mut buffer = [0_u8; 8192];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(retained.len());
        if remaining > 0 {
            retained.extend_from_slice(&buffer[..read.min(remaining)]);
        }
        if read > remaining {
            truncated = true;
        }
    }
    Ok((retained, truncated))
}

pub(crate) fn inherited_environment(names: &[&str]) -> Vec<(OsString, OsString)> {
    names
        .iter()
        .filter_map(|name| std::env::var_os(name).map(|value| (OsString::from(name), value)))
        .collect()
}

pub(crate) fn request(
    program: impl AsRef<OsStr>,
    arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    current_dir: &Path,
) -> ProcessRequest {
    ProcessRequest {
        program: program.as_ref().to_os_string(),
        arguments: arguments
            .into_iter()
            .map(|argument| argument.as_ref().to_os_string())
            .collect(),
        current_dir: current_dir.to_path_buf(),
        environment: inherited_environment(&[
            "PATH",
            "HOME",
            "USER",
            "TMPDIR",
            "RUSTUP_HOME",
            "CARGO_HOME",
            "RUSTUP_TOOLCHAIN",
        ]),
        clear_environment: true,
        stdin: None,
        timeout: Duration::from_secs(30),
        output_limit: 16 * 1024 * 1024,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_only_configured_output_prefix_while_draining() {
        let mut command = request(
            "sh",
            ["-c", "printf 1234567890; printf abcdefghij >&2"],
            Path::new("."),
        );
        command.output_limit = 4;
        let result = run(&command).expect("process succeeds");
        assert_eq!(result.stdout, b"1234");
        assert_eq!(result.stderr, b"abcd");
        assert!(result.stdout_truncated);
        assert!(result.stderr_truncated);
    }

    #[test]
    fn terminates_process_after_timeout() {
        let mut command = request("sh", ["-c", "sleep 5"], Path::new("."));
        command.timeout = Duration::from_millis(50);
        let result = run(&command).expect("timeout is an outcome");
        assert!(result.timed_out);
    }
}
