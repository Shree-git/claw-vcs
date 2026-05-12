//! Command-line interface for Claw VCS.
//!
//! The binary wires Clap parsing, runtime safety profiles, diagnostics, and
//! command dispatch for the `claw` executable.

use clap::Parser;
use tracing_subscriber::EnvFilter;

mod auth_store;
mod commands;
mod config;
mod conflict_writer;
mod diff_render;
mod error;
mod ignore;
mod merge_state;
mod output;
mod worktree;

use commands::Commands;
use commands::{ErrorFormat, RuntimeOptions};
use error::CliDiagnostic;

#[derive(clap::ValueEnum, Clone, Debug)]
enum ProfileArg {
    Dev,
    Prod,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ErrorFormatArg {
    Human,
    Json,
}

#[derive(Parser)]
#[command(
    name = "claw",
    version,
    about = "Intent-native, agent-native version control"
)]
struct Cli {
    /// Operational profile for safety defaults
    #[arg(long, value_enum, default_value_t = ProfileArg::Prod)]
    profile: ProfileArg,
    /// Validate client/server compatibility before remote operations.
    ///
    /// This is the default; the flag remains for scripts that already pass it.
    #[arg(long)]
    compat_check: bool,
    /// Skip client/server compatibility validation before remote operations
    #[arg(long)]
    no_compat_check: bool,
    /// Output command runtime errors in human or JSON envelope form
    #[arg(long, value_enum, default_value_t = ErrorFormatArg::Human)]
    error_format: ErrorFormatArg,
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let runtime = RuntimeOptions {
        profile: match cli.profile {
            ProfileArg::Dev => "dev".to_string(),
            ProfileArg::Prod => "prod".to_string(),
        },
        compat_check: cli.compat_check || !cli.no_compat_check,
        error_format: match cli.error_format {
            ErrorFormatArg::Human => ErrorFormat::Human,
            ErrorFormatArg::Json => ErrorFormat::Json,
        },
    };

    if let Err(err) = cli.command.run(&runtime).await {
        let diagnostic = CliDiagnostic::from_error(&err);
        match runtime.error_format {
            ErrorFormat::Human => {
                diagnostic.print_human();
                std::process::exit(diagnostic.exit_code);
            }
            ErrorFormat::Json => {
                let request_id = format!(
                    "req_{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_millis()
                );
                let envelope = serde_json::json!({
                    "code": diagnostic.code,
                    "message": diagnostic.message,
                    "request_id": request_id,
                    "remediation": diagnostic.remediation,
                    "exit_code": diagnostic.exit_code,
                    "details": diagnostic.details,
                });
                eprintln!("{}", serde_json::to_string(&envelope)?);
                std::process::exit(diagnostic.exit_code);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, ErrorFormatArg, ProfileArg};

    #[test]
    fn parses_global_flags_defaults() {
        let cli = Cli::parse_from(["claw", "status"]);

        assert!(matches!(cli.profile, ProfileArg::Prod));
        assert!(cli.compat_check || !cli.no_compat_check);
        assert!(matches!(cli.error_format, ErrorFormatArg::Human));
    }

    #[test]
    fn parses_global_flags_explicit_values() {
        let cli = Cli::parse_from([
            "claw",
            "--profile",
            "dev",
            "--compat-check",
            "--error-format",
            "json",
            "status",
        ]);

        assert!(matches!(cli.profile, ProfileArg::Dev));
        assert!(cli.compat_check);
        assert!(!cli.no_compat_check);
        assert!(matches!(cli.error_format, ErrorFormatArg::Json));
    }

    #[test]
    fn parses_global_no_compat_check_escape_hatch() {
        let cli = Cli::parse_from(["claw", "--no-compat-check", "status"]);

        assert!(!cli.compat_check);
        assert!(cli.no_compat_check);
    }
}
