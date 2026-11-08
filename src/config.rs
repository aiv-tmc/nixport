//! Detection of per-user settings that belong to a program.

use crate::error::Result;
use crate::paths;
use std::collections::{BTreeSet, HashSet};
use std::fs;

/// Directories below `$HOME` that commonly hold application settings and state.
const BASES: [&str; 3] = [".config", ".local/share", ".local/state"];

/// Returns the settings paths (relative to `$HOME`) whose final component
/// equals one of `names`, ignoring case, plus matching `~/.<name>` entries.
/// The cache directory is deliberately not searched.
pub fn detect(names: &[String]) -> Result<Vec<String>> {
    let home = paths::home()?;
    let wanted: HashSet<String> = names.iter().filter(|n| !n.is_empty()).map(|n| n.to_lowercase()).collect();
    let mut found = BTreeSet::new();

    for base in BASES {
        let Ok(entries) = fs::read_dir(home.join(base)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if wanted.contains(&name.to_lowercase()) {
                found.insert(format!("{base}/{name}"));
            }
        }
    }
    for name in &wanted {
        let dotted = format!(".{name}");
        if home.join(&dotted).exists() {
            found.insert(dotted);
        }
    }
    Ok(found.into_iter().collect())
}
