#![no_main]

use claw_git::importer::list_git_refs;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(content) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(tmp) = tempfile::tempdir() else {
        return;
    };
    let git_dir = tmp.path();
    let _ = std::fs::create_dir_all(git_dir.join("refs").join("heads"));
    let _ = std::fs::write(git_dir.join("packed-refs"), content);
    let _ = list_git_refs(git_dir, "refs/heads/");
});
