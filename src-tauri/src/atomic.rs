//! Atomic file writes.
//!
//! ADR-0003 makes the filesystem the source of truth for Actions, so a crash in
//! the middle of a save must never leave a truncated TOML behind. Write to a
//! temp file in the *same directory* (so the rename stays on one volume) and
//! rename over the target.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Write `contents` to `path` atomically. Returns the temp path that was used,
/// which the caller may want to feed to the watcher's self-write suppression.
pub fn write_atomic(path: &Path, contents: &str) -> io::Result<PathBuf> {
    let dir = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "path has no parent directory")
    })?;
    fs::create_dir_all(dir)?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let tmp = dir.join(format!(".{file_name}.beckon-tmp"));

    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
    }

    // Windows `rename` fails if the destination exists, so remove it first.
    // The window between the two calls is why the temp file is kept around: a
    // crash there leaves `.name.beckon-tmp` recoverable by hand.
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&tmp, path)?;
    Ok(tmp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a.toml");

        write_atomic(&target, "first").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "first");

        write_atomic(&target, "second").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "second");
    }

    #[test]
    fn leaves_no_temp_file_behind() {
        let dir = tempfile::tempdir().unwrap();
        write_atomic(&dir.path().join("a.toml"), "x").unwrap();
        let names: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["a.toml".to_string()]);
    }

    #[test]
    fn creates_missing_directories() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested").join("a.toml");
        write_atomic(&target, "x").unwrap();
        assert!(target.exists());
    }
}
