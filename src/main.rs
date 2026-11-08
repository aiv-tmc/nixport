//! Nixport entry point: parses the command line and dispatches to a sub-command.

mod cli;
mod config;
mod container;
mod desktop;
mod error;
mod icon;
mod inspect;
mod install;
mod log;
mod meta;
mod pack;
mod paths;
mod resolve;
mod run;
mod stream;

use cli::Action;
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Argument errors are reported before the output mode is known, so they
    // are always printed in the human-readable form.
    let cli = match cli::parse(env::args().skip(1).collect()) {
        Ok(cli) => cli,
        Err(e) => {
            log::error(&e.0);
            return ExitCode::from(2);
        }
    };
    log::set_json(cli.json);

    let result = match cli.action {
        Action::Pack(options) => pack::run(options),
        Action::Run { file, args, no_config } => run::run(&file, args, no_config),
        Action::Install { file, user } => install::run(&file, user),
        Action::Info { file, icon_out } => inspect::info(&file, icon_out.as_deref()),
        Action::Clean { file } => inspect::clean(file.as_deref()),
        Action::Help => {
            println!("{}", cli::USAGE);
            Ok(())
        }
        Action::Version => {
            println!("nixport {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error(&e.0);
            ExitCode::FAILURE
        }
    }
}
