// SPDX-License-Identifier: GPL-3.0-only

use std::{env, path::PathBuf, time::Duration};

use clap::{ArgAction, Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use crate::{download, error::GfileError};

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

        /// Output directory or explicit output file path.
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,

        /// Overwrite the final output file if it already exists.
        #[arg(long = "force")]
        force: bool,

        /// Per-read stall timeout in seconds.
        #[arg(long = "timeout", default_value_t = 60)]
        timeout: u64,

        /// Retry count for retryable network/server failures.
        #[arg(long = "retries", default_value_t = 3)]
        retries: u32,

        /// Override the default User-Agent.
        #[arg(long = "user-agent")]
        user_agent: Option<String>,

        /// Save the fetched download page HTML for diagnostics.
        #[arg(long = "dump-page")]
        dump_page: Option<PathBuf>,

        /// Disable progress and non-error status output.
        #[arg(short = 'q', long = "quiet")]
        quiet: bool,
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

pub async fn run(cli: Cli) -> Result<(), GfileError> {
    match cli.command {
        Commands::Download {
            url,
            output,
            force,
            timeout,
            retries,
            user_agent,
            dump_page,
            quiet,
        } => {
            let outcome = download::download(download::DownloadOptions {
                url,
                output,
                force,
                timeout: Duration::from_secs(timeout),
                retries,
                user_agent,
                dump_page,
                quiet,
                allow_any_host: test_allow_any_host(),
            })
            .await?;
            if !quiet {
                println!("{}", outcome.path.display());
            }
            Ok(())
        }
        Commands::Upload { file: _ } => {
            eprintln!("upload is not implemented yet");
            Err(GfileError::UploadRejected {
                detail: "upload command is not implemented yet".to_owned(),
            })
        }
    }
}

fn test_allow_any_host() -> bool {
    env::var("GFILE_TEST_ALLOW_ANY_HOST").as_deref() == Ok("1")
}
