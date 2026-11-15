//! Atomic file writes.
//!
//! The filesystem is the source of truth for Actions, so a crash in
//! the middle of a save must never leave a truncated TOML behind. Write to a
//! temp file in the *same directory* (so the rename stays on one volume) and
//! rename over the target.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Write `contents` to `path` atomically.
pub fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "path has no parent directory")
    })?;
    fs::create_dir_all(dir)?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let tmp = dir.join(format!(".{file_name}.beckon-tmp"));

    let written = (|| {
        {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(contents.as_bytes())?;
            file.sync_all()?;
        }

        // `fs::rename` replaces an existing destination on every platform we
        // build for, so there is no unlink-then-rename window to crash inside.
        fs::rename(&tmp, path)
    })();

    // A write that failed must not leave its scratch file behind: the target
    // being read-only or held open by an editor is an ordinary refusal, and a
    // `.name.beckon-tmp` beside every Action that was ever refused is litter in
    // a directory the user is told is theirs to browse.
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
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

    /// A directory where the file should be is the portable way to make the
    /// rename fail: neither platform replaces a directory with a file.
    #[test]
    fn a_refused_write_leaves_no_temp_file_behind() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a.toml");
        fs::create_dir(&target).unwrap();

        assert!(write_atomic(&target, "x").is_err());
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
