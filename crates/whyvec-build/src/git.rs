use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::process::{self, ProcessError};

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
}

impl From<&ChangeAtom> for ChangeAtomSummary {
    fn from(value: &ChangeAtom) -> Self {
        Self {
            id: value.id.clone(),
            display: value.display.clone(),
            paths: value.paths.clone(),
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
    pub fn apply(&self, worktree: &Path) -> Result<(), GitError> {
        match &self.payload {
            AtomPayload::Patch(patch) => {
                let mut request = process::request(
                    "git",
                    ["apply", "--binary", "--whitespace=nowarn", "-"],
                    worktree,
                );
                request.stdin = Some(patch.clone());
                let result = process::run(&request)?;
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
) -> Result<process::ProcessResult, GitError> {
    git_os(current_dir, arguments.map(OsString::from))
}

fn git_os(
    current_dir: &Path,
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<process::ProcessResult, GitError> {
    let mut request = process::request("git", arguments, current_dir);
    request.timeout = Duration::from_mins(1);
    let result = process::run(&request)?;
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
