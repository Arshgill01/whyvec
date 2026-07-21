use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use whyvec_experiment::{ProcessError, ProcessResult, process_request, run_process};

#[derive(Clone, Debug)]
enum AtomPayload {
    Patch(Vec<u8>),
    UntrackedFile {
        relative: PathBuf,
        content: Vec<u8>,
        permissions: fs::Permissions,
    },
    UntrackedSymlink {
        relative: PathBuf,
        target: PathBuf,
        target_is_dir: bool,
    },
}

#[derive(Clone, Debug)]
pub struct ChangeAtom {
    pub id: String,
    pub display: String,
    pub paths: Vec<String>,
    payload: AtomPayload,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChangeAtomSummary {
    pub id: String,
    pub display: String,
    pub paths: Vec<String>,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TextHunkSummary {
    pub id: String,
    pub parent_atom: String,
    pub file: String,
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
    pub removed_preview: Vec<String>,
    pub added_preview: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct TextHunk {
    pub summary: TextHunkSummary,
    header: Vec<u8>,
    patch: Vec<u8>,
}

impl From<&ChangeAtom> for ChangeAtomSummary {
    fn from(value: &ChangeAtom) -> Self {
        Self {
            id: value.id.clone(),
            display: value.display.clone(),
            paths: value.paths.clone(),
            sha256: value.payload_digest(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GitRepository {
    pub root: PathBuf,
    pub base_commit: String,
}

#[derive(Debug)]
pub enum GitError {
    Process(ProcessError),
    CommandFailed {
        operation: &'static str,
        stderr: String,
    },
    InvalidUtf8Path,
    InvalidNameStatus,
    UnmergedChanges,
    EmptyPatch(String),
    UnsupportedUntrackedType(PathBuf),
    ReservedPathTransition(Vec<String>),
    Io(std::io::Error),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Process(error) => error.fmt(formatter),
            Self::CommandFailed { operation, stderr } => {
                write!(formatter, "git {operation} failed: {stderr}")
            }
            Self::InvalidUtf8Path => formatter
                .write_str("a changed path is not valid UTF-8; this adapter currently declines it"),
            Self::InvalidNameStatus => {
                formatter.write_str("git returned malformed NUL-delimited name status")
            }
            Self::UnmergedChanges => {
                formatter.write_str("unmerged paths cannot be atomized safely")
            }
            Self::EmptyPatch(path) => write!(formatter, "no patch was produced for {path}"),
            Self::UnsupportedUntrackedType(path) => write!(
                formatter,
                "unsupported untracked filesystem entry: {}",
                path.display()
            ),
            Self::ReservedPathTransition(paths) => write!(
                formatter,
                "a tracked rename or copy crosses the reserved .whyvec analysis boundary: {paths:?}"
            ),
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for GitError {}

impl From<ProcessError> for GitError {
    fn from(value: ProcessError) -> Self {
        Self::Process(value)
    }
}

impl From<std::io::Error> for GitError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl GitRepository {
    pub fn discover(path: &Path, base: &str) -> Result<Self, GitError> {
        let root_output = git(path, ["rev-parse", "--show-toplevel"])?;
        let root = PathBuf::from(parse_single_line(&root_output.stdout)?).canonicalize()?;
        let base_expression = format!("{base}^{{commit}}");
        let base_output = git(&root, ["rev-parse", base_expression.as_str()])?;
        let base_commit = parse_single_line(&base_output.stdout)?;
        Ok(Self { root, base_commit })
    }

    pub fn capture_atoms(&self) -> Result<Vec<ChangeAtom>, GitError> {
        let status = git(
            &self.root,
            [
                "diff",
                "--name-status",
                "-z",
                "--find-renames",
                self.base_commit.as_str(),
                "--",
            ],
        )?;
        let mut atoms = parse_tracked_status(&status.stdout)?;
        let mut retained_atoms = Vec::with_capacity(atoms.len());
        for atom in atoms.drain(..) {
            let reserved_paths = atom
                .paths
                .iter()
                .filter(|path| is_reserved_path(Path::new(path)))
                .count();
            if reserved_paths == atom.paths.len() {
                continue;
            }
            if reserved_paths > 0 {
                return Err(GitError::ReservedPathTransition(atom.paths));
            }
            retained_atoms.push(atom);
        }
        atoms = retained_atoms;
        for atom in &mut atoms {
            let mut arguments = vec![
                OsString::from("diff"),
                OsString::from("--binary"),
                OsString::from("--full-index"),
                OsString::from("--no-ext-diff"),
                OsString::from("--unified=0"),
                OsString::from(&self.base_commit),
                OsString::from("--"),
            ];
            arguments.extend(atom.paths.iter().map(OsString::from));
            let patch = git_os(&self.root, arguments)?;
            if patch.stdout.is_empty() {
                return Err(GitError::EmptyPatch(atom.display.clone()));
            }
            atom.payload = AtomPayload::Patch(patch.stdout);
        }

        let untracked = git(
            &self.root,
            ["ls-files", "--others", "--exclude-standard", "-z"],
        )?;
        for raw_path in split_nul(&untracked.stdout) {
            let path = std::str::from_utf8(raw_path).map_err(|_| GitError::InvalidUtf8Path)?;
            let relative = PathBuf::from(path);
            if is_reserved_path(&relative) {
                continue;
            }
            if !is_safe_relative_path(&relative) {
                return Err(GitError::UnsupportedUntrackedType(relative));
            }
            let source = self.root.join(&relative);
            let metadata = fs::symlink_metadata(&source)?;
            let payload = if metadata.file_type().is_file() {
                AtomPayload::UntrackedFile {
                    relative: relative.clone(),
                    content: fs::read(&source)?,
                    permissions: metadata.permissions(),
                }
            } else if metadata.file_type().is_symlink() {
                let canonical_target = source.canonicalize()?;
                if !canonical_target.starts_with(&self.root) {
                    return Err(GitError::UnsupportedUntrackedType(relative));
                }
                AtomPayload::UntrackedSymlink {
                    relative: relative.clone(),
                    target: fs::read_link(&source)?,
                    target_is_dir: canonical_target.is_dir(),
                }
            } else {
                return Err(GitError::UnsupportedUntrackedType(relative));
            };
            atoms.push(ChangeAtom {
                id: atom_id("untracked", &[path]),
                display: format!("untracked {path}"),
                paths: vec![path.to_owned()],
                payload,
            });
        }

        atoms.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(atoms)
    }

    pub fn add_worktree(&self, path: &Path) -> Result<(), GitError> {
        let result = git_os(
            &self.root,
            [
                OsString::from("worktree"),
                OsString::from("add"),
                OsString::from("--detach"),
                OsString::from("--quiet"),
                path.as_os_str().to_os_string(),
                OsString::from(&self.base_commit),
            ],
        )?;
        if result.timed_out {
            return Err(GitError::CommandFailed {
                operation: "worktree add",
                stderr: "timed out".to_owned(),
            });
        }
        Ok(())
    }

    pub fn remove_worktree(&self, path: &Path) -> Result<(), GitError> {
        let result = git_os(
            &self.root,
            [
                OsString::from("worktree"),
                OsString::from("remove"),
                OsString::from("--force"),
                path.as_os_str().to_os_string(),
            ],
        )?;
        if result.timed_out {
            return Err(GitError::CommandFailed {
                operation: "worktree remove",
                stderr: "timed out".to_owned(),
            });
        }
        Ok(())
    }
}

impl ChangeAtom {
    pub fn captured_bytes(&self) -> Vec<u8> {
        match &self.payload {
            AtomPayload::Patch(patch) => patch.clone(),
            AtomPayload::UntrackedFile {
                content,
                permissions,
                ..
            } => {
                let mut bytes = b"whyvec-untracked-file\0".to_vec();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt as _;
                    bytes.extend_from_slice(&permissions.mode().to_le_bytes());
                }
                bytes.extend_from_slice(content);
                bytes
            }
            AtomPayload::UntrackedSymlink {
                target,
                target_is_dir,
                ..
            } => {
                let mut bytes = b"whyvec-untracked-symlink\0".to_vec();
                bytes.push(u8::from(*target_is_dir));
                bytes.extend_from_slice(target.as_os_str().as_encoded_bytes());
                bytes
            }
        }
    }

    fn payload_digest(&self) -> String {
        let bytes = self.captured_bytes();
        let digest = Sha256::digest(bytes);
        crate::hex_prefix(&digest, digest.len())
    }

    pub fn apply(&self, worktree: &Path) -> Result<(), GitError> {
        match &self.payload {
            AtomPayload::Patch(patch) => {
                let mut request = process_request(
                    "git",
                    [
                        "apply",
                        "--binary",
                        "--unidiff-zero",
                        "--whitespace=nowarn",
                        "-",
                    ],
                    worktree,
                );
                request.stdin = Some(patch.clone());
                let result = run_process(&request)?;
                if result.timed_out || result.exit_code != Some(0) {
                    return Err(GitError::CommandFailed {
                        operation: "apply",
                        stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
                    });
                }
                Ok(())
            }
            AtomPayload::UntrackedFile {
                relative,
                content,
                permissions,
            } => {
                let destination = worktree.join(relative);
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&destination, content)?;
                fs::set_permissions(&destination, permissions.clone())?;
                Ok(())
            }
            AtomPayload::UntrackedSymlink {
                relative,
                target,
                target_is_dir,
            } => {
                let destination = worktree.join(relative);
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                copy_symlink(target, &destination, *target_is_dir)?;
                Ok(())
            }
        }
    }

    #[must_use]
    pub fn text_hunks(&self) -> Vec<TextHunk> {
        let AtomPayload::Patch(patch) = &self.payload else {
            return Vec::new();
        };
        if self.paths.len() != 1 {
            return Vec::new();
        }
        parse_text_hunks(&self.id, &self.paths[0], patch)
    }
}

pub fn apply_text_hunks(hunks: &[TextHunk], worktree: &Path) -> Result<(), GitError> {
    if hunks.is_empty() {
        return Ok(());
    }
    let mut grouped = std::collections::BTreeMap::<&str, Vec<&TextHunk>>::new();
    for hunk in hunks {
        grouped
            .entry(hunk.summary.parent_atom.as_str())
            .or_default()
            .push(hunk);
    }
    let mut patch = Vec::new();
    for group in grouped.values_mut() {
        group.sort_by_key(|hunk| (hunk.summary.old_start, hunk.summary.new_start));
        patch.extend_from_slice(&group[0].header);
        for hunk in group {
            patch.extend_from_slice(&hunk.patch);
        }
    }
    let mut request = process_request(
        "git",
        [
            "apply",
            "--binary",
            "--unidiff-zero",
            "--whitespace=nowarn",
            "-",
        ],
        worktree,
    );
    request.stdin = Some(patch);
    let result = run_process(&request)?;
    if result.timed_out || result.exit_code != Some(0) {
        return Err(GitError::CommandFailed {
            operation: "apply refined hunks",
            stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
        });
    }
    Ok(())
}

fn parse_text_hunks(parent: &str, file: &str, patch: &[u8]) -> Vec<TextHunk> {
    let Ok(text) = std::str::from_utf8(patch) else {
        return Vec::new();
    };
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    let starts = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.starts_with("@@ ").then_some(index))
        .collect::<Vec<_>>();
    let Some(first) = starts.first().copied() else {
        return Vec::new();
    };
    let header = lines[..first].concat().into_bytes();
    starts
        .iter()
        .enumerate()
        .filter_map(|(position, start)| {
            let end = starts.get(position + 1).copied().unwrap_or(lines.len());
            let (old_start, old_lines, new_start, new_lines) = parse_hunk_range(lines[*start])?;
            let body = lines[*start..end].concat();
            let mut digest = Sha256::new();
            digest.update(parent.as_bytes());
            digest.update(body.as_bytes());
            let id = format!("hunk.{}", crate::hex_prefix(&digest.finalize(), 8));
            let removed_preview = preview_lines(&lines[*start + 1..end], '-');
            let added_preview = preview_lines(&lines[*start + 1..end], '+');
            Some(TextHunk {
                summary: TextHunkSummary {
                    id,
                    parent_atom: parent.to_owned(),
                    file: file.to_owned(),
                    old_start,
                    old_lines,
                    new_start,
                    new_lines,
                    removed_preview,
                    added_preview,
                },
                header: header.clone(),
                patch: body.into_bytes(),
            })
        })
        .collect()
}

fn parse_hunk_range(header: &str) -> Option<(usize, usize, usize, usize)> {
    let range = header.strip_prefix("@@ ")?.split(" @@").next()?;
    let mut parts = range.split_whitespace();
    let old = parse_range(parts.next()?.strip_prefix('-')?)?;
    let new = parse_range(parts.next()?.strip_prefix('+')?)?;
    Some((old.0, old.1, new.0, new.1))
}

fn parse_range(value: &str) -> Option<(usize, usize)> {
    let mut parts = value.split(',');
    let start = parts.next()?.parse().ok()?;
    let lines = parts.next().map_or(Some(1), |count| count.parse().ok())?;
    Some((start, lines))
}

fn preview_lines(lines: &[&str], marker: char) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| line.strip_prefix(marker))
        .take(8)
        .map(|line| line.trim_end_matches('\n').to_owned())
        .collect()
}

fn parse_tracked_status(bytes: &[u8]) -> Result<Vec<ChangeAtom>, GitError> {
    let fields = split_nul(bytes);
    let mut atoms = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let status = std::str::from_utf8(fields[index]).map_err(|_| GitError::InvalidNameStatus)?;
        index += 1;
        if status.is_empty() {
            continue;
        }
        if status.starts_with('U') {
            return Err(GitError::UnmergedChanges);
        }
        let path_count = if status.starts_with('R') || status.starts_with('C') {
            2
        } else {
            1
        };
        if index + path_count > fields.len() {
            return Err(GitError::InvalidNameStatus);
        }
        let paths = fields[index..index + path_count]
            .iter()
            .map(|path| {
                std::str::from_utf8(path)
                    .map(str::to_owned)
                    .map_err(|_| GitError::InvalidUtf8Path)
            })
            .collect::<Result<Vec<_>, _>>()?;
        index += path_count;
        let display = if paths.len() == 2 {
            format!("{} {} → {}", &status[..1], paths[0], paths[1])
        } else {
            format!("{} {}", &status[..1], paths[0])
        };
        atoms.push(ChangeAtom {
            id: atom_id(
                status,
                &paths.iter().map(String::as_str).collect::<Vec<_>>(),
            ),
            display,
            paths,
            payload: AtomPayload::Patch(Vec::new()),
        });
    }
    Ok(atoms)
}

fn atom_id(kind: &str, paths: &[&str]) -> String {
    let mut digest = Sha256::new();
    digest.update(kind.as_bytes());
    for path in paths {
        digest.update(path.len().to_le_bytes());
        digest.update(path.as_bytes());
    }
    let bytes = digest.finalize();
    let short = crate::hex_prefix(&bytes, 8);
    format!("file.{short}")
}

fn split_nul(bytes: &[u8]) -> Vec<&[u8]> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect()
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn is_reserved_path(path: &Path) -> bool {
    path.components()
        .find_map(|component| match component {
            Component::Normal(value) => Some(value),
            _ => None,
        })
        .is_some_and(|component| component == ".whyvec")
}

fn parse_single_line(bytes: &[u8]) -> Result<String, GitError> {
    let line = std::str::from_utf8(bytes).map_err(|_| GitError::InvalidUtf8Path)?;
    Ok(line.trim().to_owned())
}

fn git<const N: usize>(
    current_dir: &Path,
    arguments: [&str; N],
) -> Result<ProcessResult, GitError> {
    git_os(current_dir, arguments.map(OsString::from))
}

fn git_os(
    current_dir: &Path,
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<ProcessResult, GitError> {
    let mut request = process_request("git", arguments, current_dir);
    request.timeout = Duration::from_mins(1);
    let result = run_process(&request)?;
    if result.timed_out || result.exit_code != Some(0) {
        return Err(GitError::CommandFailed {
            operation: "command",
            stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
        });
    }
    Ok(result)
}

#[cfg(unix)]
fn copy_symlink(
    target: &Path,
    destination: &Path,
    _target_is_dir: bool,
) -> Result<(), std::io::Error> {
    std::os::unix::fs::symlink(target, destination)
}

#[cfg(windows)]
fn copy_symlink(
    target: &Path,
    destination: &Path,
    target_is_dir: bool,
) -> Result<(), std::io::Error> {
    if target_is_dir {
        std::os::windows::fs::symlink_dir(target, destination)
    } else {
        std::os::windows::fs::symlink_file(target, destination)
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn untracked_file_atom_is_an_immutable_snapshot() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "whyvec-git-snapshot-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("create repository");
        fs::write(root.join("tracked.txt"), "base\n").expect("write tracked file");
        git_test(&root, ["init", "--quiet"]);
        git_test(&root, ["config", "user.email", "whyvec@example.invalid"]);
        git_test(&root, ["config", "user.name", "WhyVec Test"]);
        git_test(&root, ["add", "tracked.txt"]);
        git_test(&root, ["commit", "--quiet", "-m", "base"]);
        fs::write(root.join("new.txt"), "captured\n").expect("write untracked file");

        let repository = GitRepository::discover(&root, "HEAD").expect("discover repository");
        let atom = repository
            .capture_atoms()
            .expect("capture atoms")
            .into_iter()
            .find(|atom| atom.paths == ["new.txt"])
            .expect("untracked atom");
        fs::write(root.join("new.txt"), "changed after capture\n").expect("mutate source");
        let destination = root.join("materialized");
        fs::create_dir(&destination).expect("create destination");
        atom.apply(&destination).expect("apply captured atom");

        assert_eq!(
            fs::read_to_string(destination.join("new.txt")).expect("read materialized file"),
            "captured\n"
        );
        fs::remove_dir_all(&root).expect("remove repository");
    }

    fn git_test<const N: usize>(root: &Path, arguments: [&str; N]) {
        let status = Command::new("git")
            .args(arguments)
            .current_dir(root)
            .status()
            .expect("run git");
        assert!(status.success());
    }
}
