use serde::Serialize;

pub mod exit_codes {
    #[allow(dead_code)]
    pub const OK: i32 = 0;
    pub const GENERAL: i32 = 1;
    #[allow(dead_code)]
    pub const USAGE: i32 = 2;
    pub const NOT_REPOSITORY: i32 = 3;
    pub const CONFIG: i32 = 4;
    pub const IO: i32 = 5;
    pub const AUTH: i32 = 6;
    pub const REMOTE: i32 = 7;
    pub const CONFLICT: i32 = 8;
    pub const WORKTREE_DIRTY: i32 = 9;
    pub const POLICY: i32 = 10;
    pub const COMPATIBILITY: i32 = 11;
}

#[derive(Debug, Clone, Serialize)]
pub struct CliDiagnostic {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<&'static str>,
    pub exit_code: i32,
    pub details: serde_json::Value,
}

impl CliDiagnostic {
    pub fn from_usage_error(message: String, kind: clap::error::ErrorKind) -> Self {
        Self {
            code: "USAGE_ERROR",
            message,
            remediation: Some("Run `claw --help` or `claw <command> --help` for usage."),
            exit_code: exit_codes::USAGE,
            details: serde_json::json!({
                "kind": format!("{kind:?}"),
            }),
        }
    }

    pub fn from_error(err: &anyhow::Error) -> Self {
        let message = err.to_string();
        let chain = err
            .chain()
            .map(|cause| cause.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let text = chain.to_lowercase();

        let is_not_repository = err
            .chain()
            .any(|cause| cause.is::<crate::config::NotRepositoryError>());

        let (code, exit_code, remediation) = if is_not_repository
            || text.contains("not in a claw repository")
            || text.contains("not a claw repository")
            || text.contains("no .claw directory found")
        {
            (
                "NOT_REPOSITORY",
                exit_codes::NOT_REPOSITORY,
                Some(
                    "Run `claw init` in this directory, or `cd` into an existing Claw repository.",
                ),
            )
        } else if text.contains("config") {
            (
                    "CONFIG_ERROR",
                    exit_codes::CONFIG,
                    Some("Run `claw doctor` to inspect repository configuration, then fix the reported file."),
                )
        } else if text.contains("no token found")
            || text.contains("authorization code")
            || text.contains("token exchange failed")
        {
            (
                    "AUTH_ERROR",
                    exit_codes::AUTH,
                    Some("Run `claw auth login --profile default`, or set a token with `claw auth token set`."),
                )
        } else if text.contains("policy") {
            (
                "POLICY_DENIED",
                exit_codes::POLICY,
                Some("Review the policy requirements, add the required evidence/signatures, or run `claw policy eval --json` for details."),
            )
        } else if text.contains("compatibility check failed") || text.contains("incompatible") {
            (
                "COMPATIBILITY_ERROR",
                exit_codes::COMPATIBILITY,
                Some("Use a compatible CLI/daemon version or rerun with `--no-compat-check` only after verifying the risk."),
            )
        } else if text.contains("remote") {
            (
                    "REMOTE_ERROR",
                    exit_codes::REMOTE,
                    Some("Run `claw remote list` to inspect configured remotes, or add one with `claw remote add`."),
                )
        } else if text.contains("uncommitted changes") {
            (
                    "WORKTREE_DIRTY",
                    exit_codes::WORKTREE_DIRTY,
                    Some("Run `claw status`, snapshot your work with `claw snapshot -m <message>`, or retry with `--force` when appropriate."),
                )
        } else if text.contains("conflict") || text.contains("merge in progress") {
            (
                    "CONFLICT_STATE",
                    exit_codes::CONFLICT,
                    Some("Run `claw resolve` to inspect conflicts, then `claw snapshot -m <message>` after resolving them."),
                )
        } else if text.contains("io error") || text.contains("permission denied") {
            (
                "IO_ERROR",
                exit_codes::IO,
                Some("Check filesystem permissions and that the target path is accessible."),
            )
        } else {
            (
                    "CLI_ERROR",
                    exit_codes::GENERAL,
                    Some("Run the command again with `--help`, or run `claw doctor` for local repository checks."),
                )
        };

        Self {
            code,
            message,
            remediation,
            exit_code,
            details: serde_json::Value::Null,
        }
    }

    pub fn print_human(&self) {
        eprintln!(
            "error[{code}]: {message}",
            code = self.code,
            message = self.message
        );
        if let Some(remediation) = self.remediation {
            eprintln!("hint: {remediation}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{exit_codes, CliDiagnostic};

    #[test]
    fn classifies_policy_failures() {
        let err = anyhow::anyhow!("policy 'release' denied revision abc");
        let diagnostic = CliDiagnostic::from_error(&err);

        assert_eq!(diagnostic.code, "POLICY_DENIED");
        assert_eq!(diagnostic.exit_code, exit_codes::POLICY);
    }

    #[test]
    fn classifies_compatibility_failures() {
        let err = anyhow::anyhow!("compatibility check failed for origin: incompatible");
        let diagnostic = CliDiagnostic::from_error(&err);

        assert_eq!(diagnostic.code, "COMPATIBILITY_ERROR");
        assert_eq!(diagnostic.exit_code, exit_codes::COMPATIBILITY);
    }

    #[test]
    fn classifies_usage_failures() {
        let diagnostic = CliDiagnostic::from_usage_error(
            "unexpected argument '--wat'".to_string(),
            clap::error::ErrorKind::UnknownArgument,
        );

        assert_eq!(diagnostic.code, "USAGE_ERROR");
        assert_eq!(diagnostic.exit_code, exit_codes::USAGE);
        assert_eq!(diagnostic.details["kind"], "UnknownArgument");
    }
}
