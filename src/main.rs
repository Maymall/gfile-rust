// SPDX-License-Identifier: GPL-3.0-only

use std::process::ExitCode;

use clap::Parser;
use gfile_rust::cli::{self, Cli};

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let code = err.exit_code();
            let _ = err.print();
            return exit_code(code);
        }
    };

    cli::init_tracing(cli.verbose);

    match cli::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{}", err.user_message());
            exit_code(i32::from(err.exit_code()))
        }
    }
}

fn exit_code(code: i32) -> ExitCode {
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
