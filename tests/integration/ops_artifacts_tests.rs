use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn workspace_path(relative: &str) -> PathBuf {
    workspace_root().join(relative)
}

fn read_workspace_file(relative: &str) -> String {
    let path = workspace_path(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

#[test]
fn compatibility_matrix_json_is_valid_and_has_required_keys() {
    let raw = read_workspace_file("docs/reference/compatibility-matrix.json");
    let json: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("compatibility matrix must parse as JSON: {err}"));

    let root = json
        .as_object()
        .expect("compatibility matrix root must be a JSON object");

    for key in [
        "schemaVersion",
        "lastUpdated",
        "components",
        "supportLevels",
        "releases",
        "relationshipRules",
    ] {
        assert!(
            root.contains_key(key),
            "compatibility matrix missing top-level key: {key}"
        );
    }

    let releases = root
        .get("releases")
        .and_then(Value::as_array)
        .expect("compatibility matrix releases must be an array");

    let declared_support_levels = root
        .get("supportLevels")
        .and_then(Value::as_array)
        .expect("compatibility matrix supportLevels must be an array");

    let allowed_support_levels: HashSet<&str> = ["full", "limited", "read-only", "unsupported"]
        .into_iter()
        .collect();

    let mut support_levels = HashSet::new();
    for (idx, level) in declared_support_levels.iter().enumerate() {
        let level = level
            .as_str()
            .unwrap_or_else(|| panic!("support level at index {idx} must be a string"));
        assert!(
            allowed_support_levels.contains(level),
            "support level at index {idx} is not allowed: {level}"
        );
        assert!(
            support_levels.insert(level.to_string()),
            "support level is duplicated: {level}"
        );
    }

    assert!(
        !releases.is_empty(),
        "compatibility matrix releases must not be empty"
    );

    let mut release_names = HashSet::new();
    let mut release_order = Vec::with_capacity(releases.len());

    for (idx, release) in releases.iter().enumerate() {
        let release_obj = release
            .as_object()
            .unwrap_or_else(|| panic!("release at index {idx} must be an object"));

        let release_name = release_obj
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("release at index {idx} must include string key: name"));

        assert!(
            release_names.insert(release_name.to_string()),
            "release name is duplicated: {release_name}"
        );
        release_order.push(release_name.to_string());

        for key in [
            "cliVersion",
            "daemonVersion",
            "policySchemaVersion",
            "storageFormatVersion",
        ] {
            assert!(
                release_obj.contains_key(key),
                "release at index {idx} missing key: {key}"
            );
        }

        let compatibility = release_obj
            .get("compatibility")
            .and_then(Value::as_array)
            .unwrap_or_else(|| {
                panic!("release at index {idx} must include array key: compatibility")
            });

        for (compat_idx, entry) in compatibility.iter().enumerate() {
            let entry_obj = entry.as_object().unwrap_or_else(|| {
                panic!(
                    "compatibility entry at release index {idx}, entry index {compat_idx} must be an object"
                )
            });

            let support = entry_obj
                .get("support")
                .and_then(Value::as_str)
                .unwrap_or_else(|| {
                    panic!(
                        "compatibility entry at release index {idx}, entry index {compat_idx} must include string key: support"
                    )
                });

            assert!(
                support_levels.contains(support),
                "compatibility entry at release index {idx}, entry index {compat_idx} has unsupported support level: {support}"
            );

            let target_release = entry_obj
                .get("targetRelease")
                .and_then(Value::as_str)
                .unwrap_or_else(|| {
                    panic!(
                        "compatibility entry at release index {idx}, entry index {compat_idx} must include string key: targetRelease"
                    )
                });

            assert!(
                !target_release.is_empty(),
                "compatibility entry at release index {idx}, entry index {compat_idx} has empty targetRelease"
            );
        }
    }

    let release_name_to_index: std::collections::HashMap<&str, usize> = release_order
        .iter()
        .enumerate()
        .map(|(idx, name)| (name.as_str(), idx))
        .collect();

    assert!(
        release_name_to_index.contains_key("N"),
        "releases must include base release N"
    );

    for (earlier, later) in [("N", "N+1"), ("N+1", "N+2")] {
        if let (Some(earlier_idx), Some(later_idx)) = (
            release_name_to_index.get(earlier),
            release_name_to_index.get(later),
        ) {
            assert!(
                earlier_idx < later_idx,
                "release order must progress forward: {earlier} must come before {later}"
            );
        }
    }

    assert!(
        !release_name_to_index.contains_key("N+2") || release_name_to_index.contains_key("N+1"),
        "release N+2 requires N+1 to be present"
    );

    for (idx, release) in releases.iter().enumerate() {
        let release_obj = release
            .as_object()
            .unwrap_or_else(|| panic!("release at index {idx} must be an object"));
        let compatibility = release_obj
            .get("compatibility")
            .and_then(Value::as_array)
            .unwrap_or_else(|| {
                panic!("release at index {idx} must include array key: compatibility")
            });

        for (compat_idx, entry) in compatibility.iter().enumerate() {
            let entry_obj = entry.as_object().unwrap_or_else(|| {
                panic!(
                    "compatibility entry at release index {idx}, entry index {compat_idx} must be an object"
                )
            });
            let target_release = entry_obj
                .get("targetRelease")
                .and_then(Value::as_str)
                .unwrap_or_else(|| {
                    panic!(
                        "compatibility entry at release index {idx}, entry index {compat_idx} must include string key: targetRelease"
                    )
                });

            assert!(
                release_name_to_index.contains_key(target_release),
                "compatibility entry at release index {idx}, entry index {compat_idx} references unknown targetRelease: {target_release}"
            );
        }
    }
}

#[test]
fn policy_and_interface_docs_exist_and_include_required_phrases() {
    let interface_path = workspace_path("docs/reference/public-interface-manifest.md");
    let deprecation_path = workspace_path("docs/reference/deprecation-policy.md");

    assert!(
        interface_path.exists(),
        "missing required artifact: {}",
        interface_path.display()
    );
    assert!(
        deprecation_path.exists(),
        "missing required artifact: {}",
        deprecation_path.display()
    );

    let interface_content = fs::read_to_string(&interface_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", interface_path.display()));
    let deprecation_content = fs::read_to_string(&deprecation_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", deprecation_path.display()));

    let combined_lower = format!("{interface_content}\n{deprecation_content}").to_lowercase();

    for phrase in ["stable", "beta", "experimental", "n+1", "n+2"] {
        assert!(
            combined_lower.contains(phrase),
            "required phrase not found in reference artifacts: {phrase}"
        );
    }

    let deprecation_lines: Vec<String> = deprecation_content
        .lines()
        .map(|line| line.trim().to_lowercase())
        .collect();

    let n_line = deprecation_lines
        .iter()
        .find(|line| line.starts_with("-") && line.contains("**n "))
        .expect("deprecation policy must include lifecycle bullet for N");
    assert!(
        n_line.contains("warn"),
        "deprecation lifecycle for N must mention warn"
    );

    let n_plus_1_line = deprecation_lines
        .iter()
        .find(|line| line.starts_with("-") && line.contains("**n+1"))
        .expect("deprecation policy must include lifecycle bullet for N+1");
    assert!(
        n_plus_1_line.contains("soft-fail"),
        "deprecation lifecycle for N+1 must mention soft-fail"
    );

    let n_plus_2_line = deprecation_lines
        .iter()
        .find(|line| line.starts_with("-") && line.contains("**n+2"))
        .expect("deprecation policy must include lifecycle bullet for N+2");
    assert!(
        n_plus_2_line.contains("remove"),
        "deprecation lifecycle for N+2 must mention remove"
    );
}

#[test]
fn public_launch_assets_exist_and_are_upload_ready() {
    for artifact in [
        "scripts/demo.sh",
        "scripts/public-launch-preflight.sh",
        "scripts/verify-release-channel.sh",
        "examples/basic-demo/scripts/demo.sh",
        "docs/assets/social-preview.png",
        "docs/assets/social-preview.svg",
        "docs/operations/public-launch-checklist.md",
        "docs/operations/backlog-coverage.md",
        "docs/operations/package-registry-strategy.md",
        ".github/workflows/release-channel-smoke.yml",
    ] {
        let path = workspace_path(artifact);
        assert!(
            path.exists(),
            "missing required public-launch artifact: {}",
            path.display()
        );
    }

    let demo_wrapper = read_workspace_file("scripts/demo.sh");
    assert!(
        demo_wrapper.contains("examples/basic-demo/scripts/demo.sh"),
        "top-level demo wrapper must delegate to the maintained basic demo"
    );

    let launch_preflight = read_workspace_file("scripts/public-launch-preflight.sh");
    for phrase in [
        "secret_scanning",
        "required_signatures",
        "https://crates.io/api/v1/crates/$crate_name",
        "claw-vcs-core",
        "ShreeGit/ClawVCS",
        "docs/assets/social-preview.png",
    ] {
        assert!(
            launch_preflight.contains(phrase),
            "public-launch preflight must include phrase: {phrase}"
        );
    }
    assert!(
        !launch_preflight.contains("mapfile"),
        "public-launch preflight must remain compatible with macOS system Bash"
    );

    let release_verifier = read_workspace_file("scripts/verify-release-channel.sh");
    for phrase in [
        "gh release download",
        "claw-installer.sh",
        "--tag \"$tag\"",
        "CLAW_VERIFY_HOMEBREW",
    ] {
        assert!(
            release_verifier.contains(phrase),
            "release-channel verifier must include phrase: {phrase}"
        );
    }

    let social_preview = fs::read(workspace_path("docs/assets/social-preview.png"))
        .expect("social preview PNG must be readable");
    assert!(
        social_preview.starts_with(b"\x89PNG\r\n\x1a\n"),
        "social preview asset must be a PNG file"
    );
    assert!(
        social_preview.len() < 1_000_000,
        "social preview PNG must stay under GitHub's 1 MB upload limit"
    );

    let launch_checklist = read_workspace_file("docs/operations/public-launch-checklist.md");
    assert!(
        launch_checklist.contains("docs/assets/social-preview.png"),
        "launch checklist must name the upload-ready social preview asset"
    );
    assert!(
        launch_checklist.contains("Package-name checks"),
        "launch checklist must record package-name verification evidence"
    );
    assert!(
        launch_checklist.contains("scripts/public-launch-preflight.sh"),
        "launch checklist must point maintainers to the public-launch preflight"
    );

    let backlog_coverage = read_workspace_file("docs/operations/backlog-coverage.md");
    for item in ["| 1 |", "| 50 |", "| 100 |", "| 110 |"] {
        assert!(
            backlog_coverage.contains(item),
            "backlog coverage must include item marker {item}"
        );
    }
    assert!(
        backlog_coverage.contains("External pending"),
        "backlog coverage must preserve external-blocker status"
    );

    let release_channel_smoke = read_workspace_file(".github/workflows/release-channel-smoke.yml");
    assert!(
        release_channel_smoke.contains("cargo-install-git-smoke:"),
        "release-channel smoke workflow must include a cargo install from Git job"
    );
    assert!(
        release_channel_smoke.contains("needs: release-metadata"),
        "cargo install from Git smoke must use the resolved release metadata"
    );
    assert!(
        release_channel_smoke.contains("--tag \"$RELEASE_TAG\""),
        "cargo install from Git smoke must install the exact release tag under validation"
    );

    let install_log = read_workspace_file("docs/operations/install-verification-log.md");
    assert!(
        install_log.contains("--tag <launch-tag>"),
        "install verification log must require tag-specific cargo install verification"
    );
    assert!(
        install_log.contains("scripts/verify-release-channel.sh <launch-tag>"),
        "install verification log must point maintainers to the clean-host helper"
    );
}

#[test]
fn packaged_proto_copies_match_workspace_proto_sources() {
    let canonical_root = workspace_path("proto/claw");
    for packaged_root in ["crates/claw-core/proto/claw", "crates/claw-sync/proto/claw"] {
        for entry in fs::read_dir(&canonical_root).expect("canonical proto directory exists") {
            let entry = entry.expect("proto directory entry is readable");
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            let source = fs::read(entry.path())
                .unwrap_or_else(|err| panic!("failed to read proto source {file_name}: {err}"));
            let packaged_path = workspace_path(packaged_root).join(file_name.as_ref());
            let packaged = fs::read(&packaged_path).unwrap_or_else(|err| {
                panic!(
                    "failed to read packaged proto {}: {err}",
                    packaged_path.display()
                )
            });
            assert_eq!(
                packaged,
                source,
                "packaged proto {} must match proto/claw/{file_name}",
                packaged_path.display()
            );
        }
    }
}
