pub mod admin;
pub mod agent;
pub mod auth;
pub mod branch;
pub mod change;
pub mod checkout;
pub mod completion;
pub mod daemon;
pub mod diff;
pub mod doctor;
pub mod git_export;
pub mod git_import;
mod git_notes;
pub mod git_roundtrip;
pub mod init;
pub mod integrate;
pub mod intent;
pub mod log;
pub mod patch;
pub mod plugin;
pub mod policy;
pub mod remote;
pub mod resolve;
pub mod ship;
pub mod show;
pub mod snapshot;
pub mod status;
pub mod sync;
pub mod version;

use clap::Subcommand;

#[derive(Clone, Debug)]
pub enum ErrorFormat {
    Human,
    Json,
}

#[derive(Clone, Debug)]
pub struct RuntimeOptions {
    pub profile: String,
    pub compat_check: bool,
    pub error_format: ErrorFormat,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Administrative operations for production deployments
    Admin(admin::AdminArgs),
    /// Initialize a new claw repository
    Init(init::InitArgs),
    /// Generate shell completion scripts
    #[command(alias = "completion")]
    Completions(completion::CompletionArgs),
    /// Run local CLI and repository diagnostics
    Doctor(doctor::DoctorArgs),
    /// Show claw version information
    Version(version::VersionArgs),
    /// Manage intents
    Intent(intent::IntentArgs),
    /// Manage changes
    Change(change::ChangeArgs),
    /// Create and apply patches
    Patch(patch::PatchArgs),
    /// Manage external plugins
    Plugin(plugin::PluginArgs),
    /// Manage policies
    Policy(policy::PolicyArgs),
    /// Sync with a remote repository
    Sync(sync::SyncArgs),
    /// Integrate changes (merge)
    Integrate(integrate::IntegrateArgs),
    /// Ship an intent (finalize, produce capsule)
    Ship(ship::ShipArgs),
    /// Manage agent registrations
    Agent(agent::AgentArgs),
    /// Run the sync daemon
    Daemon(daemon::DaemonArgs),
    /// Run the sync daemon (alias for daemon)
    Serve(daemon::DaemonArgs),
    /// Record a snapshot of the working tree
    Snapshot(snapshot::SnapshotArgs),
    /// Switch branches or restore working tree
    Checkout(checkout::CheckoutArgs),
    /// List, create, or delete branches
    Branch(branch::BranchArgs),
    /// Show revision history
    Log(log::LogArgs),
    /// Show changes between trees
    Diff(diff::DiffArgs),
    /// Export to git format
    GitExport(git_export::GitExportArgs),
    /// Import from git format
    GitImport(git_import::GitImportArgs),
    /// Verify claw -> git -> claw roundtrip integrity
    GitRoundtrip(git_roundtrip::GitRoundtripArgs),
    /// Show working tree status
    Status(status::StatusArgs),
    /// Show details of an object
    Show(show::ShowArgs),
    /// Manage merge conflicts
    Resolve(resolve::ResolveArgs),
    /// Manage remote repositories
    Remote(remote::RemoteArgs),
    /// Authenticate with hosted remote profiles
    Auth(auth::AuthArgs),
}

impl Commands {
    pub async fn run(self, runtime: &RuntimeOptions) -> anyhow::Result<()> {
        match self {
            Commands::Admin(args) => admin::run(args, runtime),
            Commands::Init(args) => init::run(args),
            Commands::Completions(args) => completion::run(args),
            Commands::Doctor(args) => doctor::run(args),
            Commands::Version(args) => version::run(args),
            Commands::Intent(args) => intent::run(args),
            Commands::Change(args) => change::run(args),
            Commands::Patch(args) => patch::run(args),
            Commands::Plugin(args) => plugin::run(args).await,
            Commands::Policy(args) => policy::run(args),
            Commands::Sync(args) => sync::run(args, runtime).await,
            Commands::Integrate(args) => integrate::run(args),
            Commands::Ship(args) => ship::run(args),
            Commands::Agent(args) => agent::run(args),
            Commands::Daemon(args) => daemon::run(args, runtime).await,
            Commands::Serve(args) => daemon::run(args, runtime).await,
            Commands::Snapshot(args) => snapshot::run(args),
            Commands::Checkout(args) => checkout::run(args),
            Commands::Branch(args) => branch::run(args),
            Commands::Log(args) => log::run(args),
            Commands::Diff(args) => diff::run(args),
            Commands::GitExport(args) => git_export::run(args),
            Commands::GitImport(args) => git_import::run(args),
            Commands::GitRoundtrip(args) => git_roundtrip::run(args),
            Commands::Status(args) => status::run(args),
            Commands::Show(args) => show::run(args),
            Commands::Resolve(args) => resolve::run(args),
            Commands::Remote(args) => remote::run(args),
            Commands::Auth(args) => auth::run(args).await,
        }
    }
}
