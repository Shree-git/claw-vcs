#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityLevel {
    Full,
    Limited,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityReport {
    pub level: CompatibilityLevel,
    pub local: String,
    pub remote: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Version {
    major: u64,
    minor: u64,
}

impl Version {
    fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim().strip_prefix('v').unwrap_or(input.trim());
        let without_prerelease = trimmed
            .split_once('-')
            .map(|(left, _)| left)
            .unwrap_or(trimmed);
        let core = without_prerelease
            .split_once('+')
            .map(|(left, _)| left)
            .unwrap_or(without_prerelease);

        let mut parts = core.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next().unwrap_or("0").parse().ok()?;
        let _patch: u64 = parts.next().unwrap_or("0").parse().ok()?;

        Some(Self { major, minor })
    }
}

pub fn classify_versions(local: &str, remote: &str) -> CompatibilityLevel {
    let Some(local_v) = Version::parse(local) else {
        return CompatibilityLevel::Unsupported;
    };
    let Some(remote_v) = Version::parse(remote) else {
        return CompatibilityLevel::Unsupported;
    };

    if local_v.major != remote_v.major {
        return CompatibilityLevel::Unsupported;
    }

    if local_v.minor == remote_v.minor {
        return CompatibilityLevel::Full;
    }

    let minor_diff = local_v.minor.abs_diff(remote_v.minor);
    if minor_diff == 1 {
        return CompatibilityLevel::Limited;
    }

    CompatibilityLevel::Unsupported
}

pub fn compatibility_report(local: &str, remote: &str) -> CompatibilityReport {
    CompatibilityReport {
        level: classify_versions(local, remote),
        local: local.to_string(),
        remote: remote.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_versions, CompatibilityLevel};

    #[test]
    fn exact_minor_match_is_full() {
        assert_eq!(
            classify_versions("1.4.0", "1.4.9"),
            CompatibilityLevel::Full
        );
    }

    #[test]
    fn n_minus_one_minor_is_limited() {
        assert_eq!(
            classify_versions("1.4.0", "1.3.8"),
            CompatibilityLevel::Limited
        );
    }

    #[test]
    fn n_plus_one_minor_is_limited() {
        assert_eq!(
            classify_versions("1.4.0", "1.5.0"),
            CompatibilityLevel::Limited
        );
    }

    #[test]
    fn major_mismatch_is_unsupported() {
        assert_eq!(
            classify_versions("1.4.0", "2.0.0"),
            CompatibilityLevel::Unsupported
        );
    }

    #[test]
    fn malformed_versions_are_unsupported() {
        assert_eq!(
            classify_versions("dev", "1.0.0"),
            CompatibilityLevel::Unsupported
        );
        assert_eq!(
            classify_versions("1.0.0", "not-a-version"),
            CompatibilityLevel::Unsupported
        );
    }

    #[test]
    fn v_prefix_and_prerelease_are_supported() {
        assert_eq!(
            classify_versions("v1.2.3", "1.2.4-rc.1"),
            CompatibilityLevel::Full
        );
    }
}
