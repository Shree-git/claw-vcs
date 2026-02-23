use similar::{ChangeTag, TextDiff};

/// Check if stdout is a terminal (for color support).
fn use_color() -> bool {
    std::io::IsTerminal::is_terminal(&std::io::stdout())
}

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

pub fn render_unified_diff(path: &str, old_bytes: &[u8], new_bytes: &[u8]) -> String {
    let old_str = String::from_utf8_lossy(old_bytes);
    let new_str = String::from_utf8_lossy(new_bytes);
    let color = use_color();

    let diff = TextDiff::from_lines(old_str.as_ref(), new_str.as_ref());

    if !color {
        let mut output = format!("--- a/{}\n+++ b/{}\n", path, path);
        output.push_str(
            &diff
                .unified_diff()
                .context_radius(3)
                .header(&format!("a/{}", path), &format!("b/{}", path))
                .to_string(),
        );
        return output;
    }

    let mut output = format!(
        "{BOLD}--- a/{path}{RESET}\n{BOLD}+++ b/{path}{RESET}\n"
    );

    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        output.push_str(&format!("{CYAN}{}{RESET}\n", hunk.header()));
        for change in hunk.iter_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    output.push_str(&format!("{RED}-{}{RESET}", change.value()));
                    if change.missing_newline() {
                        output.push('\n');
                    }
                }
                ChangeTag::Insert => {
                    output.push_str(&format!("{GREEN}+{}{RESET}", change.value()));
                    if change.missing_newline() {
                        output.push('\n');
                    }
                }
                ChangeTag::Equal => {
                    output.push(' ');
                    output.push_str(change.value());
                    if change.missing_newline() {
                        output.push('\n');
                    }
                }
            }
        }
    }

    output
}

pub fn render_json_diff(path: &str, ops: &[claw_core::types::PatchOp]) -> String {
    let color = use_color();
    let mut output = if color {
        format!("{BOLD}--- a/{path}{RESET}\n{BOLD}+++ b/{path}{RESET}\n")
    } else {
        format!("--- a/{}\n+++ b/{}\n", path, path)
    };

    for op in ops {
        output.push_str(&format!("  {} @{}: ", op.op_type, op.address));
        if let Some(old) = &op.old_data {
            if color {
                output.push_str(&format!(
                    "{RED}old={:?}{RESET} ",
                    String::from_utf8_lossy(old)
                ));
            } else {
                output.push_str(&format!("old={:?} ", String::from_utf8_lossy(old)));
            }
        }
        if let Some(new) = &op.new_data {
            if color {
                output.push_str(&format!(
                    "{GREEN}new={:?}{RESET}",
                    String::from_utf8_lossy(new)
                ));
            } else {
                output.push_str(&format!("new={:?}", String::from_utf8_lossy(new)));
            }
        }
        output.push('\n');
    }
    output
}

pub fn render_binary_diff(
    path: &str,
    old_size: usize,
    new_size: usize,
    old_hash: &str,
    new_hash: &str,
) -> String {
    format!(
        "Binary files a/{} and b/{} differ\n  old: {} bytes ({})\n  new: {} bytes ({})\n",
        path,
        path,
        old_size,
        &old_hash[..16.min(old_hash.len())],
        new_size,
        &new_hash[..16.min(new_hash.len())],
    )
}
