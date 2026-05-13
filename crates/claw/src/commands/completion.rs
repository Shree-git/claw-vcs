use clap::{Args, CommandFactory, ValueEnum};
use clap_complete::generate;
use std::io::{ErrorKind, Write};

use crate::Cli;

#[derive(Args)]
pub struct CompletionArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    shell: CompletionShell,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Elvish,
}

impl From<CompletionShell> for clap_complete::Shell {
    fn from(shell: CompletionShell) -> Self {
        match shell {
            CompletionShell::Bash => Self::Bash,
            CompletionShell::Zsh => Self::Zsh,
            CompletionShell::Fish => Self::Fish,
            CompletionShell::Powershell => Self::PowerShell,
            CompletionShell::Elvish => Self::Elvish,
        }
    }
}

pub fn run(args: CompletionArgs) -> anyhow::Result<()> {
    let mut command = Cli::command();
    let shell: clap_complete::Shell = args.shell.into();
    let mut out = BrokenPipeSafeWriter(std::io::stdout().lock());
    generate(shell, &mut command, "claw", &mut out);
    Ok(())
}

struct BrokenPipeSafeWriter<W>(W);

impl<W: Write> Write for BrokenPipeSafeWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.0.write(buf) {
            Ok(n) => Ok(n),
            Err(err) if err.kind() == ErrorKind::BrokenPipe => Ok(buf.len()),
            Err(err) => Err(err),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self.0.flush() {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::BrokenPipe => Ok(()),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use crate::Cli;

    use super::BrokenPipeSafeWriter;

    #[test]
    fn completion_metadata_comes_from_cli_definition() {
        let command = Cli::command();

        for subcommand in ["doctor", "version", "completions", "daemon", "snapshot"] {
            assert!(
                command.find_subcommand(subcommand).is_some(),
                "CLI should expose {subcommand} for generated completions"
            );
        }

        let snapshot = command
            .find_subcommand("snapshot")
            .expect("snapshot subcommand");
        assert!(
            snapshot
                .get_arguments()
                .any(|arg| arg.get_long() == Some("message")),
            "snapshot completion metadata should include --message"
        );

        let diff = command.find_subcommand("diff").expect("diff subcommand");
        for option in ["from", "to", "path"] {
            assert!(
                diff.get_arguments()
                    .any(|arg| arg.get_long() == Some(option)),
                "diff completion metadata should include --{option}"
            );
        }

        let ship = command.find_subcommand("ship").expect("ship subcommand");
        for option in ["private-file", "recipient-key"] {
            assert!(
                ship.get_arguments()
                    .any(|arg| arg.get_long() == Some(option)),
                "ship completion metadata should include --{option}"
            );
        }

        let show = command.find_subcommand("show").expect("show subcommand");
        for option in ["decrypt-private", "recipient-secret-key"] {
            assert!(
                show.get_arguments()
                    .any(|arg| arg.get_long() == Some(option)),
                "show completion metadata should include --{option}"
            );
        }
    }

    #[test]
    fn generated_bash_completion_includes_real_commands_and_options() {
        let mut command = Cli::command();
        let mut script = Vec::new();
        clap_complete::generate(
            clap_complete::Shell::Bash,
            &mut command,
            "claw",
            &mut script,
        );
        let script = String::from_utf8(script).expect("completion script is utf-8");

        for needle in [
            "doctor",
            "version",
            "completions",
            "--profile",
            "--error-format",
            "--message",
            "--from",
            "--to",
            "--path",
            "--private-file",
            "--recipient-key",
            "--decrypt-private",
            "--recipient-secret-key",
        ] {
            assert!(
                script.contains(needle),
                "generated bash completion should contain {needle}"
            );
        }
    }

    #[test]
    fn generated_completion_scripts_cover_every_documented_shell() {
        for shell in [
            clap_complete::Shell::Bash,
            clap_complete::Shell::Zsh,
            clap_complete::Shell::Fish,
            clap_complete::Shell::PowerShell,
            clap_complete::Shell::Elvish,
        ] {
            let mut command = Cli::command();
            let mut script = Vec::new();
            clap_complete::generate(shell, &mut command, "claw", &mut script);
            let script = String::from_utf8(script).expect("completion script is utf-8");

            for needle in ["doctor", "version", "completions", "error-format"] {
                assert!(
                    script.contains(needle),
                    "generated {shell:?} completion should contain {needle}"
                );
            }
        }
    }

    #[test]
    fn documented_top_level_completion_aliases_parse() {
        let completion_alias = Cli::parse_from(["claw", "completion", "bash"]);
        assert!(
            matches!(
                completion_alias.command,
                crate::commands::Commands::Completions(_)
            ),
            "`claw completion <shell>` should parse as the completions command"
        );

        let serve_alias = Cli::parse_from(["claw", "serve"]);
        assert!(
            matches!(serve_alias.command, crate::commands::Commands::Serve(_)),
            "`claw serve` should parse as the daemon alias"
        );
    }

    #[test]
    fn completion_output_ignores_broken_pipe() {
        struct BrokenPipeWriter;

        impl std::io::Write for BrokenPipeWriter {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "closed pipe",
                ))
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let mut writer = BrokenPipeSafeWriter(BrokenPipeWriter);
        assert_eq!(
            std::io::Write::write(&mut writer, b"complete me")
                .expect("broken pipe should not be fatal"),
            b"complete me".len()
        );
    }
}
