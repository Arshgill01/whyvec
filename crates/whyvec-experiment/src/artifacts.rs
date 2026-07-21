use std::fs;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ArtifactReference {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub media_type: String,
}

#[derive(Clone, Debug)]
pub struct ArtifactStore {
    root: PathBuf,
}

#[derive(Debug)]
pub enum ArtifactError {
    Io(std::io::Error),
    UnsafePath(String),
    IntegrityMismatch(String),
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::UnsafePath(path) => write!(formatter, "unsafe artifact path {path}"),
            Self::IntegrityMismatch(path) => {
                write!(formatter, "digest or size mismatch for {path}")
            }
        }
    }
}

impl std::error::Error for ArtifactError {}

impl From<std::io::Error> for ArtifactError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl ArtifactStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Retains bytes at a repository-independent relative path using create-new semantics.
    ///
    /// # Errors
    ///
    /// Returns `ArtifactError` for an unsafe path or when the file cannot be created and synced.
    pub fn retain(
        &self,
        relative: &str,
        bytes: &[u8],
        media_type: &str,
    ) -> Result<ArtifactReference, ArtifactError> {
        let relative_path = safe_relative(relative)?;
        let destination = self.root.join(relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        write_new(&destination, bytes)?;
        Ok(ArtifactReference {
            path: relative.replace('\\', "/"),
            sha256: sha256(bytes),
            size: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            media_type: media_type.to_owned(),
        })
    }

    /// Verifies every declared artifact without following an escaping relative path.
    ///
    /// # Errors
    ///
    /// Returns `ArtifactError` when a path is unsafe, unavailable, or differs in size or digest.
    pub fn verify(&self, artifacts: &[ArtifactReference]) -> Result<(), ArtifactError> {
        for artifact in artifacts {
            let relative = safe_relative(&artifact.path)?;
            let bytes = fs::read(self.root.join(relative))?;
            if sha256(&bytes) != artifact.sha256
                || u64::try_from(bytes.len()).ok() != Some(artifact.size)
            {
                return Err(ArtifactError::IntegrityMismatch(artifact.path.clone()));
            }
        }
        Ok(())
    }

    /// Makes all retained regular files read-only after the owning report is finalized.
    ///
    /// # Errors
    ///
    /// Returns `ArtifactError` when traversal or a permission update fails.
    pub fn finalize_read_only(&self) -> Result<(), ArtifactError> {
        let mut pending = vec![self.root.clone()];
        while let Some(path) = pending.pop() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let file_type = entry.file_type()?;
                if file_type.is_dir() {
                    pending.push(entry.path());
                } else if file_type.is_file() {
                    let mut permissions = entry.metadata()?.permissions();
                    permissions.set_readonly(true);
                    fs::set_permissions(entry.path(), permissions)?;
                }
            }
        }
        Ok(())
    }

    /// Writes a non-manifest file with the same non-overwriting durability contract.
    ///
    /// # Errors
    ///
    /// Returns `ArtifactError` if the file already exists or cannot be written and synced.
    pub fn write_new(&self, relative: &str, bytes: &[u8]) -> Result<(), ArtifactError> {
        let relative = safe_relative(relative)?;
        let destination = self.root.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        write_new(&destination, bytes)
    }
}

fn safe_relative(value: &str) -> Result<&Path, ArtifactError> {
    let path = Path::new(value);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(ArtifactError::UnsafePath(value.to_owned()));
    }
    Ok(path)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), ArtifactError> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            use std::fmt::Write as _;
            write!(text, "{byte:02x}").expect("writing to String cannot fail");
            text
        })
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TemporaryDirectory(PathBuf);

    impl TemporaryDirectory {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "whyvec-artifact-test-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create artifact root");
            Self(path)
        }
    }

    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn retains_verifies_and_detects_tampering() {
        let temporary = TemporaryDirectory::new();
        let store = ArtifactStore::new(&temporary.0);
        let artifact = store
            .retain("runs/a/stdout.txt", b"observed", "text/plain")
            .expect("retain artifact");
        store
            .verify(std::slice::from_ref(&artifact))
            .expect("verify");
        fs::write(temporary.0.join(&artifact.path), b"changed").expect("tamper");
        assert!(matches!(
            store.verify(&[artifact]),
            Err(ArtifactError::IntegrityMismatch(_))
        ));
    }

    #[test]
    fn rejects_parent_traversal_before_writing() {
        let temporary = TemporaryDirectory::new();
        let store = ArtifactStore::new(&temporary.0);
        assert!(matches!(
            store.retain("../escape", b"no", "text/plain"),
            Err(ArtifactError::UnsafePath(_))
        ));
    }

    #[test]
    fn create_new_prevents_artifact_overwrite() {
        let temporary = TemporaryDirectory::new();
        let store = ArtifactStore::new(&temporary.0);
        store
            .retain("inputs/value", b"first", "text/plain")
            .expect("first write");
        assert!(matches!(
            store.retain("inputs/value", b"second", "text/plain"),
            Err(ArtifactError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists
        ));
    }

    #[test]
    fn finalization_marks_retained_files_read_only() {
        let temporary = TemporaryDirectory::new();
        let store = ArtifactStore::new(&temporary.0);
        let artifact = store
            .retain("runs/value", b"final", "text/plain")
            .expect("retain artifact");
        store.finalize_read_only().expect("finalize store");
        assert!(
            fs::metadata(temporary.0.join(artifact.path))
                .expect("artifact metadata")
                .permissions()
                .readonly()
        );
    }
}
