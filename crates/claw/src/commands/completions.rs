use clap::Args;
use clap::CommandFactory;
use clap_complete::{generate, Shell};

#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    shell: Shell,
}

pub fn run(args: CompletionsArgs) {
    let mut cmd = crate::Cli::command();
    generate(args.shell, &mut cmd, "claw", &mut std::io::stdout());
}
