//! `nixport run`: starts a packed program without installing it.
//!
//! The closure is imported once into a private chroot store below the user's
//! cache directory. The program then starts inside a bubblewrap mount
//! namespace in which that store is overlaid on `/nix/store`, so every
//! absolute store path in the program's binaries keeps working while the host
//! store stays untouched.

use crate::container::{self, Layout};
use crate::error::{fail, Context, Result};
use crate::log;
use crate::meta::{check_system, ARGS_MARKER};
use crate::paths;
use crate::stream;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Fails early when bubblewrap cannot overlay directories.
fn require_overlay() -> Result<()> {
    let out = Command::new("bwrap")
        .arg("--help")
        .output()
        .context("cannot run bwrap (bubblewrap is required)")?;
    let help = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if help.contains("--overlay-src") {
        Ok(())
    } else {
        fail("the installed bubblewrap has no overlay support; version 0.11 or newer is required")
    }
}

/// Imports the closure into the chroot store at `root` unless that was
/// already done. A `.ready` marker is written only after a complete import.
fn ensure_store(file: &Path, layout: &Layout, root: &Path) -> Result<()> {
    let marker = root.join(".ready");
    if marker.exists() {
        return Ok(());
    }
    paths::remove_tree(root)?;
    fs::create_dir_all(root).context(&format!("cannot create {}", root.display()))?;
    log::event("first launch: unpacking the program", &[]);

    let mut import = Command::new("nix-store");
    import
        .arg("--store")
        .arg(root)
        .arg("--import")
        .env_remove("NIX_REMOTE")
        .stdout(Stdio::null());
    if let Err(e) = stream::unpack(file, layout.store_off, None, import) {
        paths::remove_tree(root).ok();
        return Err(e);
    }
    fs::write(&marker, b"").context("cannot write the ready marker")
}

/// Restores the embedded settings into a per-archive data directory on first
/// use and returns the bubblewrap arguments that bind each settings path over
/// its place in `$HOME`. Settings of the host are never modified, and changes
/// made by the program persist in the data directory.
fn config_mounts(file: &Path, layout: &Layout) -> Result<Vec<String>> {
    let config_paths = &layout.meta.config_paths;
    if config_paths.is_empty() {
        return Ok(Vec::new());
    }
    for rel in config_paths {
        paths::safe_relative(rel)?;
    }
    let base: PathBuf = paths::data_dir()?.join(&layout.meta.id);
    let store = base.join("config");
    let marker = base.join(".restored");
    if !marker.exists() {
        fs::create_dir_all(&store).context(&format!("cannot create {}", store.display()))?;
        if layout.config_len > 0 {
            let mut tar = Command::new("tar");
            tar.arg("-xf").arg("-").arg("-C").arg(&store);
            stream::unpack(file, layout.config_off, Some(layout.config_len), tar)?;
        }
        fs::write(&marker, b"").context("cannot write the restore marker")?;
    }

    let home = paths::home()?;
    let mut args = Vec::new();
    for rel in config_paths {
        let source = store.join(rel);
        let target = home.join(rel);
        if !source.exists() {
            continue;
        }
        // Bind mounts need an existing destination of the same kind.
        if source.is_dir() {
            fs::create_dir_all(&target).context(&format!("cannot create {}", target.display()))?;
        } else if !target.exists() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).context(&format!("cannot create {}", parent.display()))?;
            }
            fs::write(&target, b"").context(&format!("cannot create {}", target.display()))?;
        }
        args.push("--bind".to_string());
        args.push(source.to_string_lossy().into_owned());
        args.push(target.to_string_lossy().into_owned());
    }
    Ok(args)
}

/// Unpacks the archive if needed and replaces this process with the program.
/// Returns only when the program could not be started.
pub fn run(file: &Path, program_args: Vec<String>, no_config: bool) -> Result<()> {
    let layout = container::read(file)?;
    check_system(&layout.meta)?;
    require_overlay()?;

    let root = paths::cache_dir()?.join(&layout.meta.id);
    ensure_store(file, &layout, &root)?;
    let mounts = if no_config { Vec::new() } else { config_mounts(file, &layout)? };

    // Substitute the user's arguments for the marker in the stored command line.
    let argv: Vec<String> = layout
        .meta
        .exec
        .iter()
        .flat_map(|word| {
            if word == ARGS_MARKER {
                program_args.clone()
            } else {
                vec![word.clone()]
            }
        })
        .collect();

    // The host root stays visible; only /nix/store is replaced by an overlay
    // whose lower layers are the host store and the unpacked closure.
    let error = Command::new("bwrap")
        .args(["--die-with-parent", "--bind", "/", "/", "--dev-bind", "/dev", "/dev"])
        .args(&mounts)
        .args(["--overlay-src", "/nix/store", "--overlay-src"])
        .arg(root.join("nix/store"))
        .args(["--ro-overlay", "/nix/store", "--setenv", "NIXPORT_ID"])
        .arg(&layout.meta.id)
        .arg("--")
        .args(&argv)
        .exec();
    fail(format!("cannot start bwrap: {error}"))
}
