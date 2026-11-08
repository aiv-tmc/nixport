//! Icon lookup inside store paths and the system profile.

use std::fs;
use std::path::{Path, PathBuf};

/// Directories deeper than this are not searched; icon themes are shallow.
const MAX_DEPTH: u32 = 8;

/// Calls `on_file` for every file below `dir`, following symlinked directories.
fn visit(dir: &Path, depth: u32, on_file: &mut dyn FnMut(&Path)) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit(&path, depth + 1, on_file);
        } else {
            on_file(&path);
        }
    }
}

/// Ranks candidates: scalable icons win, otherwise the largest raster size
/// found in a `<width>x<height>` path component.
fn score(path: &Path) -> u64 {
    if path.extension().is_some_and(|e| e == "svg") {
        return 1_000_000;
    }
    path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .filter_map(|c| c.split_once('x'))
        .filter_map(|(w, h)| h.parse::<u64>().ok().and(w.parse::<u64>().ok()))
        .max()
        .unwrap_or(1)
}

/// Finds the best SVG or PNG file for the `Icon=` value of a desktop entry.
/// An absolute value is used directly; a theme name is searched in the
/// `share/icons` and `share/pixmaps` directories below every root.
pub fn find(icon: &str, roots: &[String]) -> Option<PathBuf> {
    let direct = Path::new(icon);
    if direct.is_absolute() {
        return direct.is_file().then(|| direct.to_path_buf());
    }
    let wanted = [format!("{icon}.svg"), format!("{icon}.png")];
    let mut best: Option<(u64, PathBuf)> = None;
    for root in roots {
        for sub in ["share/icons", "share/pixmaps"] {
            visit(&Path::new(root).join(sub), 0, &mut |path: &Path| {
                let Some(file) = path.file_name().and_then(|n| n.to_str()) else {
                    return;
                };
                if !wanted.iter().any(|w| w == file) {
                    return;
                }
                let candidate = score(path);
                if best.as_ref().is_none_or(|(top, _)| candidate > *top) {
                    best = Some((candidate, path.to_path_buf()));
                }
            });
        }
    }
    best.map(|(_, path)| path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalable_icons_outrank_large_bitmaps() {
        assert!(score(Path::new("icons/scalable/apps/a.svg")) > score(Path::new("icons/512x512/apps/a.png")));
    }

    #[test]
    fn larger_bitmaps_outrank_smaller_ones() {
        assert!(score(Path::new("icons/256x256/apps/a.png")) > score(Path::new("icons/48x48/apps/a.png")));
    }
}
