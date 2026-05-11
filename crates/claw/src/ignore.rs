use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

pub struct IgnoreRules {
    globs: GlobSet,
}

impl IgnoreRules {
    pub fn load(repo_root: &Path) -> Self {
        let mut builder = GlobSetBuilder::new();

        // Always-ignore
        add_glob(&mut builder, ".claw/**");
        add_glob(&mut builder, ".claw");
        add_glob(&mut builder, ".git/**");
        add_glob(&mut builder, ".git");

        // Load .clawignore
        let ignore_path = repo_root.join(".clawignore");
        if ignore_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&ignore_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    // Support directory patterns: "target/" -> "target/**"
                    let pattern = if line.ends_with('/') {
                        format!("{}**", line)
                    } else {
                        line.to_string()
                    };
                    add_glob(&mut builder, &pattern);
                    // Also match the directory itself
                    if line.ends_with('/') {
                        add_glob(&mut builder, line.trim_end_matches('/'));
                    }
                }
            }
        }

        let globs = builder.build().unwrap_or_else(|_| GlobSet::empty());
        Self { globs }
    }

    pub fn is_ignored(&self, rel_path: &str, _is_dir: bool) -> bool {
        self.globs.is_match(rel_path)
    }
}

fn add_glob(builder: &mut GlobSetBuilder, pattern: &str) {
    if let Ok(glob) = Glob::new(pattern) {
        let _ = builder.add(glob);
    }
}
