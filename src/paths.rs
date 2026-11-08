//! Filesystem locations and small filesystem helpers.

use crate::error::{fail, Context, Error, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The current user's home directory, taken from `$HOME`.
pub fn home() -> Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or_else(|| Error("HOME is not set to an absolute path".into()))
}

/// Resolves `$<var>/nixport`, falling back to `~/<fallback>/nixport`.
fn xdg(var: &str, fallback: &str) -> Result<PathBuf> {
    match env::var_os(var).filter(|v| !v.is_empty()) {
        Some(base) => Ok(PathBuf::from(base).join("nixport")),
        None => Ok(home()?.join(fallback).join("nixport")),
    }
}

/// Directory that holds the unpacked programs (one sub-directory per archive id).
pub fn cache_dir() -> Result<PathBuf> {
    xdg("XDG_CACHE_HOME", ".cache")
}

/// Directory that holds the portable settings (one sub-directory per archive id).
pub fn data_dir() -> Result<PathBuf> {
    xdg("XDG_DATA_HOME", ".local/share")
}

/// Rejects absolute paths and paths that climb out of their base directory.
pub fn safe_relative(rel: &str) -> Result<()> {
    let path = Path::new(rel);
    if rel.is_empty() || path.is_absolute() || path.components().any(|c| c.as_os_str() == "..") {
        return fail(format!("unsafe relative path: {rel}"));
    }
    Ok(())
}

/// Deletes a directory tree. Store paths are read-only, so write permission
/// is restored first; a missing directory is not an error.
pub fn remove_tree(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    Command::new("chmod")
        .arg("-R")
        .arg("u+w")
        .arg(path)
        .status()
        .context("cannot run chmod")?;
    fs::remove_dir_all(path).context(&format!("cannot remove {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_are_validated() {
        assert!(safe_relative(".config/app").is_ok());
        assert!(safe_relative("../escape").is_err());
        assert!(safe_relative("/etc/passwd").is_err());
        assert!(safe_relative("").is_err());
    }
}
