//! Turns a user-supplied target into a concrete program inside `/nix/store`.
//!
//! A target is a `.desktop` file, a command name, a path inside the store,
//! a package directory, or a flake reference such as `nixpkgs#hello`.

use crate::desktop::{exec_argv, parse_desktop};
use crate::error::{fail, Error, Result};
use crate::meta::ARGS_MARKER;
use crate::stream::capture;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const STORE_PREFIX: &str = "/nix/store/";

/// A program that is ready to be packed.
pub struct Target {
    /// Display name: the desktop entry `Name`, or the program file name.
    pub name: String,
    /// Command line; contains `ARGS_MARKER` where user arguments go.
    pub argv: Vec<String>,
    /// Store paths that the closure is computed from.
    pub roots: Vec<String>,
    /// Keys of the `[Desktop Entry]` group; empty when no entry was found.
    pub desktop: HashMap<String, String>,
    /// File name of the desktop entry without extension, when one was given.
    pub desktop_stem: Option<String>,
}

/// Returns the `/nix/store/<hash>-<name>` prefix of `path`, if it has one.
pub fn store_root(path: &str) -> Option<&str> {
    let rest = path.strip_prefix(STORE_PREFIX)?;
    let end = rest.find('/').unwrap_or(rest.len());
    let (hash, name) = rest[..end].split_once('-')?;
    let valid_hash = hash.len() == 32 && hash.bytes().all(|b| b.is_ascii_digit() || b.is_ascii_lowercase());
    (valid_hash && !name.is_empty()).then(|| &path[..STORE_PREFIX.len() + end])
}

/// The package name of a store root, without hash and version suffix.
pub fn pname(root: &str) -> String {
    let entry = &root[STORE_PREFIX.len() + 33..];
    let bytes = entry.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'-' && bytes[i + 1].is_ascii_digit() {
            return entry[..i].to_string();
        }
    }
    entry.to_string()
}

/// Looks `command` up in `$PATH`; names that contain a slash are used as given.
pub fn which(command: &str) -> Option<PathBuf> {
    if command.contains('/') {
        return Some(PathBuf::from(command));
    }
    env::split_paths(&env::var_os("PATH")?)
        .map(|dir| dir.join(command))
        .find(|candidate| candidate.is_file())
}

/// Removes `.` and `..` components without touching the filesystem.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// Follows symlinks one hop at a time until the path enters `/nix/store`.
/// The hop-by-hop walk keeps the name of the final link, which matters for
/// programs that behave differently depending on their own name.
pub fn find_in_store(program: &str) -> Result<String> {
    let start = which(program).ok_or_else(|| Error(format!("program not found: {program}")))?;
    let mut current = if start.is_absolute() {
        start
    } else {
        env::current_dir()?.join(start)
    };
    for _ in 0..40 {
        let text = current.to_string_lossy().into_owned();
        if store_root(&text).is_some() {
            return Ok(text);
        }
        let Ok(link) = fs::read_link(&current) else {
            break;
        };
        current = normalize(&current.parent().unwrap_or(Path::new("/")).join(link));
    }
    let real = fs::canonicalize(&current)?.to_string_lossy().into_owned();
    if store_root(&real).is_some() {
        Ok(real)
    } else {
        fail(format!("{program} is not located in /nix/store"))
    }
}

/// Picks a program from `<package>/bin`: the explicit `exec` name, the one
/// named `hint`, or the only file there.
fn program_in(package: &Path, hint: &str, exec: Option<&str>) -> Result<String> {
    let bin = package.join("bin");
    if let Some(name) = exec {
        return Ok(bin.join(name).to_string_lossy().into_owned());
    }
    let entries = fs::read_dir(&bin).map_err(|_| Error(format!("{} has no bin directory", package.display())))?;
    let mut names: Vec<String> = entries.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    if names.iter().any(|n| n == hint) {
        return Ok(bin.join(hint).to_string_lossy().into_owned());
    }
    match names.as_slice() {
        [only] => Ok(bin.join(only).to_string_lossy().into_owned()),
        _ => fail(format!(
            "{} holds several programs ({}); choose one with --exec",
            package.display(),
            names.join(", ")
        )),
    }
}

/// Builds a flake reference and returns its main program.
fn built_program(flake: &str, exec: Option<&str>) -> Result<String> {
    let mut build = Command::new("nix");
    build
        .args([
            "build",
            "--no-link",
            "--print-out-paths",
            "--extra-experimental-features",
            "nix-command flakes",
        ])
        .arg(flake);
    let output = capture(&mut build)?;
    let out = output
        .lines()
        .next()
        .ok_or_else(|| Error("nix build printed no output path".into()))?;
    program_in(Path::new(out), flake.rsplit('#').next().unwrap_or(""), exec)
}

/// The first desktop entry shipped by the package that owns `program`;
/// empty when the package has none.
fn package_desktop(program: &str) -> HashMap<String, String> {
    let Ok(real) = fs::canonicalize(program) else {
        return HashMap::new();
    };
    let real = real.to_string_lossy().into_owned();
    let Some(root) = store_root(&real) else {
        return HashMap::new();
    };
    let Ok(entries) = fs::read_dir(Path::new(root).join("share/applications")) else {
        return HashMap::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "desktop"))
        .collect();
    files.sort();
    // An unreadable entry only costs the optional metadata, not the packing.
    files.first().and_then(|f| parse_desktop(f).ok()).unwrap_or_default()
}

/// Resolves `target` into a `Target`. `exec` selects a binary when a package
/// ships several.
pub fn resolve(target: &str, exec: Option<&str>) -> Result<Target> {
    let (mut argv, desktop, desktop_stem) = if target.ends_with(".desktop") && Path::new(target).is_file() {
        let desktop = parse_desktop(Path::new(target))?;
        let line = desktop
            .get("Exec")
            .ok_or_else(|| Error("the desktop entry has no Exec key".into()))?;
        let mut argv = exec_argv(line);
        if argv.is_empty() {
            return fail("the Exec key of the desktop entry is empty");
        }
        argv[0] = find_in_store(&argv[0])?;
        let stem = Path::new(target).file_stem().map(|s| s.to_string_lossy().into_owned());
        (argv, desktop, stem)
    } else {
        let program = if target.contains('#') {
            built_program(target, exec)?
        } else if Path::new(target).is_dir() {
            let package = fs::canonicalize(target)?;
            let root = store_root(&package.to_string_lossy())
                .map(pname)
                .ok_or_else(|| Error(format!("{target} is not a store path")))?;
            program_in(&package, &root, exec)?
        } else {
            find_in_store(target)?
        };
        let desktop = package_desktop(&program);
        (vec![program, ARGS_MARKER.to_string()], desktop, None)
    };
    if !argv.iter().any(|a| a == ARGS_MARKER) {
        argv.push(ARGS_MARKER.to_string());
    }

    let file_name = Path::new(&argv[0])
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = desktop.get("Name").cloned().unwrap_or(file_name);

    // The program path may be a symlink into another package, so both the
    // link location and its final destination contribute a root.
    let real = fs::canonicalize(&argv[0])
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut roots: Vec<String> = Vec::new();
    for candidate in [argv[0].as_str(), real.as_str()] {
        if let Some(root) = store_root(candidate) {
            if !roots.iter().any(|r| r == root) {
                roots.push(root.to_string());
            }
        }
    }
    if roots.is_empty() {
        return fail("cannot determine the package that owns the program");
    }
    Ok(Target {
        name,
        argv,
        roots,
        desktop,
        desktop_stem,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_path(rest: &str) -> String {
        format!("/nix/store/{}-{rest}", "a".repeat(32))
    }

    #[test]
    fn store_root_cuts_the_package_directory() {
        let path = store_path("hello-2.12/bin/hello");
        assert_eq!(store_root(&path), Some(store_path("hello-2.12").as_str()));
    }

    #[test]
    fn store_root_rejects_non_store_paths() {
        assert_eq!(store_root("/usr/bin/hello"), None);
        assert_eq!(store_root("/nix/store/short-hello"), None);
        assert_eq!(store_root(&format!("/nix/store/{}-", "a".repeat(32))), None);
    }

    #[test]
    fn pname_drops_the_version() {
        assert_eq!(pname(&store_path("hello-2.12.1")), "hello");
        assert_eq!(pname(&store_path("foo-bar-1.0")), "foo-bar");
        assert_eq!(pname(&store_path("unversioned")), "unversioned");
    }
}
