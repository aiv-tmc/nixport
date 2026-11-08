//! `nixport info` and `nixport clean`: archive inspection and cache cleanup.

use crate::container;
use crate::error::{fail, Context, Result};
use crate::log;
use crate::paths;
use std::fs;
use std::path::{Path, PathBuf};

/// Prints the manifest of `file` as one JSON object on stdout. With
/// `icon_out` the embedded icon is also written to that path.
pub fn info(file: &Path, icon_out: Option<&Path>) -> Result<()> {
    let layout = container::read(file)?;
    if let Some(out) = icon_out {
        if layout.icon.is_empty() {
            return fail("the archive has no icon");
        }
        fs::write(out, &layout.icon).context(&format!("cannot write {}", out.display()))?;
    }
    println!("{}", layout.meta.to_json());
    Ok(())
}

/// Removes the unpacked program of `file`, or of every archive when no file
/// is given. Portable settings are kept because they hold user data.
pub fn clean(file: Option<&Path>) -> Result<()> {
    let cache = paths::cache_dir()?;
    let candidates: Vec<PathBuf> = match file {
        Some(file) => vec![cache.join(container::read(file)?.meta.id)],
        None => match fs::read_dir(&cache) {
            Ok(entries) => entries.flatten().map(|e| e.path()).collect(),
            Err(_) => Vec::new(),
        },
    };
    let present: Vec<PathBuf> = candidates.into_iter().filter(|p| p.exists()).collect();
    for path in &present {
        paths::remove_tree(path)?;
    }
    log::event(&format!("removed {} unpacked program(s)", present.len()), &[]);
    Ok(())
}
