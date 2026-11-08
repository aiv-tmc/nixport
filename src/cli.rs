//! Command-line parsing.

use crate::error::{fail, Error, Result};
use crate::pack::Options;
use std::path::PathBuf;

pub const USAGE: &str = "\
Usage: nixport [--json] <command> [options]

Commands:
  pack TARGET [-o PATH] [--exec NAME] [--no-config] [--config-path REL]... [--level 1-19]
        Pack a program into a single .nixport file. TARGET is a .desktop
        file, a command, a store path, or a flake reference (nixpkgs#hello).
        PATH may be a file or a directory such as a mounted USB stick.
  run [--no-config] FILE [ARGS...]
        Start a packed program without installing it.
  install [--user NAME] FILE
        Install a packed program into the system (asks for privileges).
  info FILE [--icon-out PATH]
        Print the manifest as JSON; optionally save the embedded icon.
  clean [FILE]
        Delete unpacked programs from the cache.

Global option:
  --json    Report progress and errors as JSON lines.";

pub enum Action {
    Pack(Options),
    Run { file: PathBuf, args: Vec<String>, no_config: bool },
    Install { file: PathBuf, user: Option<String> },
    Info { file: PathBuf, icon_out: Option<PathBuf> },
    Clean { file: Option<PathBuf> },
    Help,
    Version,
}

pub struct Cli {
    pub json: bool,
    pub action: Action,
}

type Args = std::vec::IntoIter<String>;

/// Takes the value that follows `flag`.
fn value(args: &mut Args, flag: &str) -> Result<String> {
    args.next().ok_or_else(|| Error(format!("{flag} needs a value")))
}

/// Takes the single positional FILE argument that every non-pack command needs.
fn require_file(file: Option<PathBuf>, command: &str) -> Result<PathBuf> {
    file.ok_or_else(|| Error(format!("{command} needs a FILE argument")))
}

fn parse_pack(mut args: Args) -> Result<Action> {
    let mut options = Options {
        target: String::new(),
        output: None,
        exec: None,
        no_config: false,
        config_paths: Vec::new(),
        level: 10,
    };
    let mut target = None;
    while let Some(arg) = args.next() {
        if arg == "-o" || arg == "--output" {
            options.output = Some(PathBuf::from(value(&mut args, &arg)?));
        } else if arg == "--exec" {
            options.exec = Some(value(&mut args, &arg)?);
        } else if arg == "--no-config" {
            options.no_config = true;
        } else if arg == "--config-path" {
            options.config_paths.push(value(&mut args, &arg)?);
        } else if arg == "--level" {
            let level = value(&mut args, &arg)?;
            options.level = level.parse().map_err(|_| Error(format!("invalid level: {level}")))?;
            if !(1..=19).contains(&options.level) {
                return fail("the level must be between 1 and 19");
            }
        } else if arg.starts_with('-') {
            return fail(format!("unknown option for pack: {arg}"));
        } else if target.replace(arg).is_some() {
            return fail("pack takes exactly one TARGET");
        }
    }
    options.target = target.ok_or_else(|| Error("pack needs a TARGET argument".into()))?;
    Ok(Action::Pack(options))
}

fn parse_run(mut args: Args) -> Result<Action> {
    let mut no_config = false;
    let mut file = None;
    // Options end at the FILE; everything after it belongs to the program.
    for arg in args.by_ref() {
        if arg == "--no-config" {
            no_config = true;
        } else if arg.starts_with('-') {
            return fail(format!("unknown option for run: {arg}"));
        } else {
            file = Some(PathBuf::from(arg));
            break;
        }
    }
    let mut program_args: Vec<String> = args.collect();
    if program_args.first().is_some_and(|a| a == "--") {
        program_args.remove(0);
    }
    Ok(Action::Run {
        file: require_file(file, "run")?,
        args: program_args,
        no_config,
    })
}

fn parse_install(mut args: Args) -> Result<Action> {
    let mut user = None;
    let mut file = None;
    while let Some(arg) = args.next() {
        if arg == "--user" {
            user = Some(value(&mut args, &arg)?);
        } else if arg.starts_with('-') {
            return fail(format!("unknown option for install: {arg}"));
        } else {
            file = Some(PathBuf::from(arg));
        }
    }
    Ok(Action::Install { file: require_file(file, "install")?, user })
}

fn parse_info(mut args: Args) -> Result<Action> {
    let mut icon_out = None;
    let mut file = None;
    while let Some(arg) = args.next() {
        if arg == "--icon-out" {
            icon_out = Some(PathBuf::from(value(&mut args, &arg)?));
        } else if arg.starts_with('-') {
            return fail(format!("unknown option for info: {arg}"));
        } else {
            file = Some(PathBuf::from(arg));
        }
    }
    Ok(Action::Info { file: require_file(file, "info")?, icon_out })
}

/// Parses the arguments that follow the program name.
pub fn parse(args: Vec<String>) -> Result<Cli> {
    let mut args = args.into_iter();
    let mut json = false;
    let mut command = args.next();
    if command.as_deref() == Some("--json") {
        json = true;
        command = args.next();
    }
    let action = match command.as_deref() {
        None | Some("help") | Some("-h") | Some("--help") => Action::Help,
        Some("-V") | Some("--version") => Action::Version,
        Some("pack") => parse_pack(args)?,
        Some("run") => parse_run(args)?,
        Some("install") => parse_install(args)?,
        Some("info") => parse_info(args)?,
        Some("clean") => Action::Clean { file: args.next().map(PathBuf::from) },
        Some(other) => return fail(format!("unknown command: {other}\n\n{USAGE}")),
    };
    Ok(Cli { json, action })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn run_forwards_arguments_after_the_file() {
        let cli = parse(words(&["run", "--no-config", "app.nixport", "--", "-x"])).unwrap();
        match cli.action {
            Action::Run { file, args, no_config } => {
                assert_eq!(file, PathBuf::from("app.nixport"));
                assert_eq!(args, words(&["-x"]));
                assert!(no_config);
            }
            _ => panic!("expected the run action"),
        }
    }

    #[test]
    fn json_flag_precedes_the_command() {
        assert!(parse(words(&["--json", "clean"])).unwrap().json);
    }

    #[test]
    fn pack_requires_a_target_and_a_sane_level() {
        assert!(parse(words(&["pack"])).is_err());
        assert!(parse(words(&["pack", "x", "--level", "30"])).is_err());
        assert!(parse(words(&["pack", "x", "--level", "5", "-o", "out"])).is_ok());
    }

    #[test]
    fn unknown_commands_are_rejected() {
        assert!(parse(words(&["frobnicate"])).is_err());
    }
}
