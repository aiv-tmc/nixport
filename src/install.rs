//! `nixport install`: installs a packed program into the system.
//!
//! The closure is imported into the real `/nix/store`, the embedded settings
//! are restored into the user's home without overwriting existing files, and
//! the program is added to the user's Nix profile so that its launcher entry
//! and binaries become available everywhere.

use crate::container::{self, Layout};
use crate::error::{fail, Context, Error, Result};
use crate::log;
use crate::meta::check_system;
use crate::resolve::which;
use crate::stream::{self, capture};
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The absolute path of the nixport launcher as the user started it. The
/// launcher (not the raw binary) is needed because it provides `PATH`.
fn launcher() -> Result<PathBuf> {
    let argv0 = std::env::args().next().ok_or_else(|| Error("cannot determine the program name".into()))?;
    let found = which(&argv0).ok_or_else(|| Error(format!("cannot locate {argv0}")))?;
    Ok(fs::canonicalize(found)?)
}

/// Re-executes this command as root through polkit. Returns only when
/// pkexec could not be started.
fn elevate(file: &Path, user: Option<String>) -> Result<()> {
    let user = match user {
        Some(user) => user,
        None => capture(Command::new("id").arg("-un"))?,
    };
    let setuid_pkexec = Path::new("/run/wrappers/bin/pkexec");
    let pkexec = if setuid_pkexec.exists() { setuid_pkexec.to_path_buf() } else { which("pkexec").unwrap_or_else(|| PathBuf::from("pkexec")) };

    let mut cmd = Command::new(pkexec);
    cmd.arg(launcher()?);
    if log::is_json() {
        cmd.arg("--json");
    }
    cmd.arg("install").arg("--user").arg(&user).arg(fs::canonicalize(file)?);
    let error = cmd.exec();
    fail(format!("cannot start pkexec: {error}"))
}

/// The home directory of `user` according to the passwd database.
fn home_of(user: &str) -> Result<String> {
    let entry = capture(Command::new("getent").arg("passwd").arg(user))?;
    entry.split(':').nth(5).map(str::to_string).ok_or_else(|| Error(format!("cannot find the home directory of {user}")))
}

/// Prepares a command that runs `program` as `user` with that user's
/// `HOME` and `USER`.
fn as_user(user: &str, home: &str, program: &str) -> Command {
    let mut cmd = Command::new("runuser");
    cmd.arg("-u").arg(user).arg("--").arg("env").arg(format!("HOME={home}")).arg(format!("USER={user}")).arg(program);
    cmd
}

/// Runs `cmd` and reports whether it exited successfully.
fn succeeds(cmd: &mut Command) -> Result<bool> {
    Ok(cmd.status().context("cannot run the command")?.success())
}

/// Runs the privileged part: import, settings, profile entry.
fn install_as_root(file: &Path, layout: &Layout, user: &str) -> Result<()> {
    let home = home_of(user)?;

    log::event("importing the program into /nix/store", &[]);
    let mut import = Command::new("nix-store");
    import.arg("--import").stdout(Stdio::null());
    stream::unpack(file, layout.store_off, None, import)?;

    if layout.config_len > 0 {
        log::event("restoring settings; existing files are kept", &[]);
        let mut tar = as_user(user, &home, "tar");
        tar.arg("-xf").arg("-").arg("-C").arg(&home).arg("--skip-old-files");
        stream::unpack(file, layout.config_off, Some(layout.config_len), tar)?;
    }

    log::event("adding the program to the user profile", &[]);
    let main = &layout.meta.roots[0];
    let mut profile = as_user(user, &home, "nix");
    profile.args(["profile", "install", "--extra-experimental-features", "nix-command flakes"]).arg(main);
    if succeeds(&mut profile)? {
        return Ok(());
    }
    // Profiles created by `nix-env` cannot be modified by `nix profile`.
    let mut legacy = as_user(user, &home, "nix-env");
    legacy.arg("-i").arg(main);
    if succeeds(&mut legacy)? {
        Ok(())
    } else {
        fail("the program was imported into /nix/store but could not be added to the user profile")
    }
}

/// Installs `file`. When not running as root the command first re-executes
/// itself through pkexec on behalf of `user` (the current user by default).
pub fn run(file: &Path, user: Option<String>) -> Result<()> {
    let layout = container::read(file)?;
    check_system(&layout.meta)?;
    if capture(Command::new("id").arg("-u"))? != "0" {
        return elevate(file, user);
    }
    let user = user.ok_or_else(|| Error("--user is required when running as root".into()))?;
    install_as_root(file, &layout, &user)?;
    log::event(&format!("{} is installed", layout.meta.name), &[]);
    Ok(())
}
