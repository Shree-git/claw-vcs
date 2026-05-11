use clap::{Args, ValueEnum};

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

const COMMANDS: &[(&str, &str)] = &[
    ("admin", "Administrative operations"),
    ("agent", "Manage agent registrations"),
    ("auth", "Authenticate with ClawLab remotes"),
    ("branch", "List, create, or delete branches"),
    ("change", "Manage changes"),
    ("checkout", "Switch branches or restore the working tree"),
    ("completions", "Generate shell completion scripts"),
    ("daemon", "Run the sync daemon"),
    ("diff", "Show changes between trees"),
    ("doctor", "Run local diagnostics"),
    ("git-export", "Export to git format"),
    ("git-import", "Import from git format"),
    ("git-roundtrip", "Verify claw/git roundtrip integrity"),
    ("init", "Initialize a new claw repository"),
    ("integrate", "Integrate changes"),
    ("intent", "Manage intents"),
    ("log", "Show revision history"),
    ("patch", "Create and apply patches"),
    ("plugin", "Manage external plugins"),
    ("policy", "Manage policies"),
    ("remote", "Manage remote repositories"),
    ("resolve", "Manage merge conflicts"),
    ("serve", "Run the sync daemon"),
    ("ship", "Ship an intent"),
    ("show", "Show details of an object"),
    ("snapshot", "Record a snapshot of the working tree"),
    ("status", "Show working tree status"),
    ("sync", "Sync with a remote repository"),
    ("version", "Show version information"),
];

pub fn run(args: CompletionArgs) -> anyhow::Result<()> {
    let script = match args.shell {
        CompletionShell::Bash => bash_completion(),
        CompletionShell::Zsh => zsh_completion(),
        CompletionShell::Fish => fish_completion(),
        CompletionShell::Powershell => powershell_completion(),
        CompletionShell::Elvish => elvish_completion(),
    };

    print!("{script}");
    Ok(())
}

fn command_words() -> String {
    COMMANDS
        .iter()
        .map(|(command, _)| *command)
        .collect::<Vec<_>>()
        .join(" ")
}

fn bash_completion() -> String {
    let commands = command_words();
    format!(
        r#"# bash completion for claw
_claw()
{{
    local cur prev commands global_opts
    COMPREPLY=()
    cur="${{COMP_WORDS[COMP_CWORD]}}"
    prev="${{COMP_WORDS[COMP_CWORD-1]}}"
    commands="{commands}"
    global_opts="-h --help -V --version --profile --compat-check --no-compat-check --error-format"

    case "$prev" in
        --profile)
            COMPREPLY=( $(compgen -W "dev prod" -- "$cur") )
            return 0
            ;;
        --error-format)
            COMPREPLY=( $(compgen -W "human json" -- "$cur") )
            return 0
            ;;
        completions|completion)
            COMPREPLY=( $(compgen -W "bash zsh fish powershell elvish" -- "$cur") )
            return 0
            ;;
    esac

    if [[ $COMP_CWORD -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "$commands $global_opts" -- "$cur") )
    fi
}}
complete -F _claw claw
"#
    )
}

fn zsh_completion() -> String {
    let command_entries = COMMANDS
        .iter()
        .map(|(command, description)| format!("    '{}:{}'", command, description))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"#compdef claw
_claw() {{
  local -a commands
  commands=(
{command_entries}
  )

  _arguments -C \
    '(-h --help)'{{-h,--help}}'[Show help]' \
    '(-V --version)'{{-V,--version}}'[Show version]' \
    '--profile[Operational profile]:profile:(dev prod)' \
    '--compat-check[Validate client/server compatibility]' \
    '--no-compat-check[Skip client/server compatibility validation]' \
    '--error-format[Runtime error format]:format:(human json)' \
    '1:command:->commands' \
    '*::arg:->args'

  case $state in
    commands) _describe -t commands 'claw command' commands ;;
    args)
      case $words[2] in
        completions|completion) _values 'shell' bash zsh fish powershell elvish ;;
      esac
      ;;
  esac
}}

_claw "$@"
"#
    )
}

fn fish_completion() -> String {
    let mut script = String::from(
        "# fish completion for claw\n\
         complete -c claw -f\n\
         complete -c claw -l help -s h -d 'Show help'\n\
         complete -c claw -l version -s V -d 'Show version'\n\
         complete -c claw -l profile -xa 'dev prod' -d 'Operational profile'\n\
         complete -c claw -l compat-check -d 'Validate client/server compatibility'\n\
         complete -c claw -l no-compat-check -d 'Skip client/server compatibility validation'\n\
         complete -c claw -l error-format -xa 'human json' -d 'Runtime error format'\n",
    );
    for (command, description) in COMMANDS {
        script.push_str(&format!(
            "complete -c claw -n '__fish_use_subcommand' -a '{}' -d '{}'\n",
            command, description
        ));
    }
    script.push_str(
        "complete -c claw -n '__fish_seen_subcommand_from completions completion' -a 'bash zsh fish powershell elvish'\n",
    );
    script
}

fn powershell_completion() -> String {
    let commands = command_words();
    format!(
        r#"# PowerShell completion for claw
Register-ArgumentCompleter -Native -CommandName claw -ScriptBlock {{
    param($wordToComplete, $commandAst, $cursorPosition)
    $commands = "{commands}".Split(" ")
    $profiles = "dev", "prod"
    $formats = "human", "json"
    $shells = "bash", "zsh", "fish", "powershell", "elvish"
    $words = $commandAst.CommandElements | ForEach-Object {{ $_.Extent.Text }}

    $candidates = if ($words[-1] -eq "--profile") {{
        $profiles
    }} elseif ($words[-1] -eq "--error-format") {{
        $formats
    }} elseif ($words -contains "completions" -or $words -contains "completion") {{
        $shells
    }} else {{
        $commands + "--help" + "--version" + "--profile" + "--compat-check" + "--no-compat-check" + "--error-format"
    }}

    $candidates |
        Where-Object {{ $_ -like "$wordToComplete*" }} |
        ForEach-Object {{ [System.Management.Automation.CompletionResult]::new($_, $_, "ParameterValue", $_) }}
}}
"#
    )
}

fn elvish_completion() -> String {
    let commands = COMMANDS
        .iter()
        .map(|(command, _)| format!("'{command}'"))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        r#"# elvish completion for claw
set edit:completion:arg-completer[claw] = {{|@words|
    var commands = [{commands}]
    if (<= (count $words) 2) {{
        put $@commands --help --version --profile --compat-check --no-compat-check --error-format
    }} elif (or (has-value $words completions) (has-value $words completion)) {{
        put bash zsh fish powershell elvish
    }}
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::{bash_completion, fish_completion};

    #[test]
    fn completions_include_launch_commands() {
        let bash = bash_completion();
        assert!(bash.contains("doctor"));
        assert!(bash.contains("version"));
        assert!(bash.contains("completions"));

        let fish = fish_completion();
        assert!(fish.contains("claw -l profile"));
    }
}
