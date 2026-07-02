// SPDX-License-Identifier: GPL-3.0-only

use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use crate::error::GfileError;

#[derive(Debug, Parser)]
#[command(
    name = "gfile",
    version,
    about = "Upload and download GigaFile public web files",
    long_about = None
)]
pub struct Cli {
    #[arg(
        short = 'v',
        long = "verbose",
        action = ArgAction::Count,
        global = true,
        help = "Increase logging verbosity (-v for info, -vv for debug)"
    )]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Download a file from a public GigaFile page.
    Download {
        /// Download page URL.
        url: String,
    },
    /// Upload a local file.
    Upload {
        /// File to upload.
        file: PathBuf,
    },
}

pub fn init_tracing(verbosity: u8) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let level = match verbosity {
            0 => "warn",
            1 => "info",
            _ => "debug",
        };
        EnvFilter::new(format!("gfile_rust={level},gfile={level}"))
    });

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

pub fn run(cli: Cli) -> Result<(), GfileError> {
    match cli.command {
        Commands::Download { url: _ } => {
            eprintln!("download is not implemented yet");
            Err(GfileError::Parse {
                what: "download command is not implemented yet".to_owned(),
                hint: "This M0 bootstrap intentionally contains no transfer implementation."
                    .to_owned(),
            })
        }
        Commands::Upload { file: _ } => {
            eprintln!("upload is not implemented yet");
            Err(GfileError::UploadRejected {
                detail: "upload command is not implemented yet".to_owned(),
            })
        }
    }
}
