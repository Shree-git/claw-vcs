use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use claw_core::id::{ChangeId, ObjectId};
use claw_core::object::Object;
use claw_core::types::{Blob, FileMode, Revision, Tree, TreeEntry};
use claw_store::ClawStore;

use crate::error::GitImportError;

pub struct GitImporter<'a> {
    store: &'a ClawStore,
    /// Maps git SHA-1 -> claw ObjectId
    object_map: HashMap<[u8; 20], ObjectId>,
}

impl<'a> GitImporter<'a> {
    pub fn new(store: &'a ClawStore) -> Self {
        Self {
            store,
            object_map: HashMap::new(),
        }
    }

    pub fn get_object_id(&self, git_sha1: &[u8; 20]) -> Option<ObjectId> {
        self.object_map.get(git_sha1).copied()
    }

    /// Import a git ref and write it to a claw ref.
    pub fn import_ref(
        &mut self,
        git_dir: &Path,
        git_ref: &str,
        claw_ref: &str,
    ) -> Result<ObjectId, GitImportError> {
        let commit_sha1 = resolve_git_ref(git_dir, git_ref)?;
        let revision_id = self.import_commit(git_dir, &commit_sha1)?;
        self.store.set_ref(claw_ref, &revision_id)?;
        self.import_change_refs(git_dir)?;
        Ok(revision_id)
    }

    fn import_change_refs(&mut self, git_dir: &Path) -> Result<(), GitImportError> {
        let changes_dir = git_dir.join("refs").join("claw").join("changes");
        if changes_dir.exists() {
            self.import_change_refs_from_dir(git_dir, &changes_dir)?;
        }

        let packed_refs = git_dir.join("packed-refs");
        if packed_refs.exists() {
            let content = fs::read_to_string(&packed_refs)?;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') || line.starts_with('^') {
                    continue;
                }
                let mut parts = line.split_whitespace();
                let Some(sha_hex) = parts.next() else {
                    continue;
                };
                let Some(name) = parts.next() else {
                    continue;
                };

                if let Some(change_id) = name.strip_prefix("refs/claw/changes/") {
                    let sha1 = sha1_from_hex(sha_hex)?;
                    let revision_id = self.import_commit(git_dir, &sha1)?;
                    self.store
                        .set_ref(&format!("changes/{}", change_id), &revision_id)?;
                }
            }
        }

        Ok(())
    }

    fn import_change_refs_from_dir(
        &mut self,
        git_dir: &Path,
        dir: &Path,
    ) -> Result<(), GitImportError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                self.import_change_refs_from_dir(git_dir, &path)?;
                continue;
            }

            if !path.is_file() {
                continue;
            }

            let Some(change_id) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            let sha_hex = fs::read_to_string(&path)?;
            let sha1 = sha1_from_hex(sha_hex.trim())?;
            let revision_id = self.import_commit(git_dir, &sha1)?;
            self.store
                .set_ref(&format!("changes/{}", change_id), &revision_id)?;
        }

        Ok(())
    }

    fn import_commit(
        &mut self,
        git_dir: &Path,
        commit_sha1: &[u8; 20],
    ) -> Result<ObjectId, GitImportError> {
        if let Some(id) = self.object_map.get(commit_sha1) {
            return Ok(*id);
        }

        let (kind, data) = read_git_object(git_dir, commit_sha1)?;
        if kind != "commit" {
            return Err(GitImportError::InvalidGitObject(format!(
                "expected commit, got {kind}"
            )));
        }

        let parsed = parse_commit(&data)?;

        let tree_id = self.import_tree(git_dir, &parsed.tree_sha1)?;

        let mut parent_ids = Vec::with_capacity(parsed.parents.len());
        for parent_sha1 in &parsed.parents {
            parent_ids.push(self.import_commit(git_dir, parent_sha1)?);
        }

        let revision = Revision {
            change_id: parsed.change_id,
            parents: parent_ids,
            patches: Vec::new(),
            snapshot_base: None,
            tree: Some(tree_id),
            capsule_id: None,
            author: parsed.author,
            created_at_ms: parsed.created_at_ms,
            summary: parsed.summary,
            policy_evidence: Vec::new(),
        };

        let object_id = self.store.store_object(&Object::Revision(revision))?;
        self.object_map.insert(*commit_sha1, object_id);
        Ok(object_id)
    }

    fn import_tree(
        &mut self,
        git_dir: &Path,
        tree_sha1: &[u8; 20],
    ) -> Result<ObjectId, GitImportError> {
        if let Some(id) = self.object_map.get(tree_sha1) {
            return Ok(*id);
        }

        let (kind, data) = read_git_object(git_dir, tree_sha1)?;
        if kind != "tree" {
            return Err(GitImportError::InvalidGitObject(format!(
                "expected tree, got {kind}"
            )));
        }

        let mut idx = 0usize;
        let mut entries = Vec::new();
        while idx < data.len() {
            let mode_end_rel = data[idx..]
                .iter()
                .position(|b| *b == b' ')
                .ok_or_else(|| GitImportError::InvalidGitObject("invalid tree mode".into()))?;
            let mode_end = idx + mode_end_rel;
            let mode = &data[idx..mode_end];
            idx = mode_end + 1;

            let name_end_rel = data[idx..]
                .iter()
                .position(|b| *b == 0)
                .ok_or_else(|| GitImportError::InvalidGitObject("invalid tree name".into()))?;
            let name_end = idx + name_end_rel;
            let name_bytes = &data[idx..name_end];
            let name = String::from_utf8(name_bytes.to_vec())
                .map_err(|e| GitImportError::InvalidGitObject(e.to_string()))?;
            idx = name_end + 1;

            if idx + 20 > data.len() {
                return Err(GitImportError::InvalidGitObject(
                    "tree entry missing SHA-1".into(),
                ));
            }
            let mut child_sha1 = [0u8; 20];
            child_sha1.copy_from_slice(&data[idx..idx + 20]);
            idx += 20;

            let file_mode = match mode {
                b"100644" => FileMode::Regular,
                b"100755" => FileMode::Executable,
                b"120000" => FileMode::Symlink,
                b"40000" | b"040000" => FileMode::Directory,
                b"160000" => {
                    return Err(GitImportError::UnsupportedType(
                        "git submodule entries (160000) are not supported".into(),
                    ));
                }
                _ => {
                    return Err(GitImportError::InvalidGitObject(format!(
                        "unsupported tree mode: {}",
                        String::from_utf8_lossy(mode)
                    )));
                }
            };

            let object_id = match file_mode {
                FileMode::Directory => self.import_tree(git_dir, &child_sha1)?,
                _ => self.import_blob(git_dir, &child_sha1)?,
            };

            entries.push(TreeEntry {
                name,
                mode: file_mode,
                object_id,
            });
        }

        let object_id = self.store.store_object(&Object::Tree(Tree { entries }))?;
        self.object_map.insert(*tree_sha1, object_id);
        Ok(object_id)
    }

    fn import_blob(
        &mut self,
        git_dir: &Path,
        blob_sha1: &[u8; 20],
    ) -> Result<ObjectId, GitImportError> {
        if let Some(id) = self.object_map.get(blob_sha1) {
            return Ok(*id);
        }

        let (kind, data) = read_git_object(git_dir, blob_sha1)?;
        if kind != "blob" {
            return Err(GitImportError::InvalidGitObject(format!(
                "expected blob, got {kind}"
            )));
        }

        let blob = Blob {
            data,
            media_type: None,
        };
        let object_id = self.store.store_object(&Object::Blob(blob))?;
        self.object_map.insert(*blob_sha1, object_id);
        Ok(object_id)
    }
}

struct ParsedCommit {
    tree_sha1: [u8; 20],
    parents: Vec<[u8; 20]>,
    author: String,
    created_at_ms: u64,
    summary: String,
    change_id: Option<ChangeId>,
}

fn parse_commit(data: &[u8]) -> Result<ParsedCommit, GitImportError> {
    let text = std::str::from_utf8(data)
        .map_err(|e| GitImportError::InvalidGitObject(format!("commit not utf-8: {e}")))?;

    let (headers, message) = text.split_once("\n\n").ok_or_else(|| {
        GitImportError::InvalidGitObject("commit missing header/body split".into())
    })?;

    let mut tree_sha1 = None;
    let mut parents = Vec::new();
    let mut author = String::from("Unknown");
    let mut created_at_ms = 0u64;

    for line in headers.lines() {
        if let Some(value) = line.strip_prefix("tree ") {
            tree_sha1 = Some(sha1_from_hex(value)?);
        } else if let Some(value) = line.strip_prefix("parent ") {
            parents.push(sha1_from_hex(value)?);
        } else if let Some(value) = line.strip_prefix("author ") {
            let (name, ts_ms) = parse_author(value);
            author = name;
            created_at_ms = ts_ms;
        }
    }

    let summary = message
        .lines()
        .next()
        .map(str::trim_end)
        .filter(|s| !s.is_empty())
        .unwrap_or("(imported git commit)")
        .to_string();

    let change_id = parse_claw_change_id(message);

    Ok(ParsedCommit {
        tree_sha1: tree_sha1
            .ok_or_else(|| GitImportError::InvalidGitObject("commit missing tree".into()))?,
        parents,
        author,
        created_at_ms,
        summary,
        change_id,
    })
}

fn parse_author(author_line: &str) -> (String, u64) {
    let mut parts = author_line.rsplitn(3, ' ');
    let _tz = parts.next();
    let ts = parts
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let name_and_email = parts.next().unwrap_or(author_line).trim();

    let name = if let Some(idx) = name_and_email.rfind(" <") {
        name_and_email[..idx].trim().to_string()
    } else {
        name_and_email.to_string()
    };

    let name = if name.is_empty() {
        "Unknown".to_string()
    } else {
        name
    };

    (name, ts.saturating_mul(1000))
}

fn parse_claw_change_id(message: &str) -> Option<ChangeId> {
    for line in message.lines().rev() {
        if let Some(value) = line.strip_prefix("Claw-Change: ") {
            if let Ok(change_id) = ChangeId::from_string(value.trim()) {
                return Some(change_id);
            }
        }
    }
    None
}

fn read_git_object(git_dir: &Path, sha1: &[u8; 20]) -> Result<(String, Vec<u8>), GitImportError> {
    let hex = hex::encode(sha1);
    let path = git_dir.join("objects").join(&hex[..2]).join(&hex[2..]);
    if !path.exists() {
        return Err(GitImportError::ObjectNotFound(format!(
            "git object not found: {hex}"
        )));
    }

    let compressed = fs::read(&path)?;
    let mut decoder = flate2::read::ZlibDecoder::new(compressed.as_slice());
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    let nul_pos = decompressed
        .iter()
        .position(|b| *b == 0)
        .ok_or_else(|| GitImportError::InvalidGitObject("missing git object header".into()))?;

    let header = std::str::from_utf8(&decompressed[..nul_pos])
        .map_err(|e| GitImportError::InvalidGitObject(e.to_string()))?;
    let (kind, size_str) = header
        .split_once(' ')
        .ok_or_else(|| GitImportError::InvalidGitObject("invalid git object header".into()))?;
    let size = size_str
        .parse::<usize>()
        .map_err(|e| GitImportError::InvalidGitObject(e.to_string()))?;

    let data = decompressed[nul_pos + 1..].to_vec();
    if data.len() != size {
        return Err(GitImportError::InvalidGitObject(format!(
            "git object size mismatch: header={size}, actual={}",
            data.len()
        )));
    }

    Ok((kind.to_string(), data))
}

fn resolve_git_ref(git_dir: &Path, git_ref: &str) -> Result<[u8; 20], GitImportError> {
    let mut candidates: Vec<String> = Vec::new();
    if git_ref.starts_with("refs/") {
        candidates.push(git_ref.to_string());
    } else {
        candidates.push(git_ref.to_string());
        candidates.push(format!("refs/{git_ref}"));
        candidates.push(format!("refs/heads/{git_ref}"));
    }

    for candidate in &candidates {
        let path = git_dir.join(candidate);
        if path.exists() {
            let content = fs::read_to_string(path)?;
            return sha1_from_hex(content.trim());
        }
    }

    let packed_refs = git_dir.join("packed-refs");
    if packed_refs.exists() {
        let content = fs::read_to_string(packed_refs)?;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('^') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let Some(sha_hex) = parts.next() else {
                continue;
            };
            let Some(name) = parts.next() else {
                continue;
            };
            if candidates.iter().any(|c| c == name) {
                return sha1_from_hex(sha_hex);
            }
        }
    }

    Err(GitImportError::ObjectNotFound(format!(
        "git ref not found: {git_ref}"
    )))
}

fn sha1_from_hex(hex_str: &str) -> Result<[u8; 20], GitImportError> {
    let bytes = hex::decode(hex_str)?;
    let arr: [u8; 20] = bytes
        .try_into()
        .map_err(|_| GitImportError::InvalidGitObject("expected 20-byte SHA-1".into()))?;
    Ok(arr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use claw_core::object::Object;
    use claw_core::types::FileMode;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;
    use tempfile::tempdir;

    fn git_object_bytes(kind: &str, body: &[u8]) -> Vec<u8> {
        let header = format!("{kind} {}\0", body.len());
        let mut out = Vec::with_capacity(header.len() + body.len());
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(body);
        out
    }

    fn write_git_object(git_dir: &Path, kind: &str, body: &[u8]) -> [u8; 20] {
        let obj = git_object_bytes(kind, body);
        let sha1 = crate::blob_convert::git_sha1(&obj);
        let hex = hex::encode(sha1);
        let obj_dir = git_dir.join("objects").join(&hex[..2]);
        fs::create_dir_all(&obj_dir).unwrap();
        let obj_path = obj_dir.join(&hex[2..]);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&obj).unwrap();
        let compressed = encoder.finish().unwrap();
        fs::write(obj_path, compressed).unwrap();
        sha1
    }

    fn write_git_ref(git_dir: &Path, git_ref: &str, sha1: &[u8; 20]) {
        let path = git_dir.join(git_ref);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, format!("{}\n", hex::encode(sha1))).unwrap();
    }

    #[test]
    fn imports_commit_tree_and_blob() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        let git_dir = root.join(".git");
        fs::create_dir_all(git_dir.join("objects")).unwrap();

        let blob_sha1 = write_git_object(&git_dir, "blob", b"hello\n");

        let mut tree_body = Vec::new();
        tree_body.extend_from_slice(b"100644 hello.txt\0");
        tree_body.extend_from_slice(&blob_sha1);
        let tree_sha1 = write_git_object(&git_dir, "tree", &tree_body);

        let commit_body = format!(
            "tree {}\nauthor Alice <alice@example.com> 1700000000 +0000\ncommitter Alice <alice@example.com> 1700000000 +0000\n\nInitial import\n",
            hex::encode(tree_sha1)
        );
        let commit_sha1 = write_git_object(&git_dir, "commit", commit_body.as_bytes());
        write_git_ref(&git_dir, "refs/heads/main", &commit_sha1);

        let store = ClawStore::init(root).unwrap();
        let mut importer = GitImporter::new(&store);
        let rev_id = importer
            .import_ref(&git_dir, "refs/heads/main", "heads/imported")
            .unwrap();

        assert_eq!(store.get_ref("heads/imported").unwrap(), Some(rev_id));

        let rev = match store.load_object(&rev_id).unwrap() {
            Object::Revision(r) => r,
            other => panic!("expected revision, got {other:?}"),
        };
        assert_eq!(rev.summary, "Initial import");
        assert_eq!(rev.author, "Alice");
        assert_eq!(rev.created_at_ms, 1_700_000_000_000);
        assert!(rev.parents.is_empty());

        let tree_id = rev.tree.unwrap();
        let tree = match store.load_object(&tree_id).unwrap() {
            Object::Tree(t) => t,
            other => panic!("expected tree, got {other:?}"),
        };
        assert_eq!(tree.entries.len(), 1);
        assert_eq!(tree.entries[0].name, "hello.txt");
        assert_eq!(tree.entries[0].mode, FileMode::Regular);

        let blob = match store.load_object(&tree.entries[0].object_id).unwrap() {
            Object::Blob(b) => b,
            other => panic!("expected blob, got {other:?}"),
        };
        assert_eq!(blob.data, b"hello\n");
    }

    #[test]
    fn deterministic_mapping_for_same_git_input() {
        fn import_once() -> (ObjectId, ObjectId, ObjectId) {
            let tmp = tempdir().unwrap();
            let root = tmp.path();
            let git_dir = root.join(".git");
            fs::create_dir_all(git_dir.join("objects")).unwrap();

            let blob_sha1 = write_git_object(&git_dir, "blob", b"same content");

            let mut tree_body = Vec::new();
            tree_body.extend_from_slice(b"100644 file.txt\0");
            tree_body.extend_from_slice(&blob_sha1);
            let tree_sha1 = write_git_object(&git_dir, "tree", &tree_body);

            let commit_body = format!(
                "tree {}\nauthor Bob <bob@example.com> 1700000100 +0000\ncommitter Bob <bob@example.com> 1700000100 +0000\n\nDeterministic\n",
                hex::encode(tree_sha1)
            );
            let commit_sha1 = write_git_object(&git_dir, "commit", commit_body.as_bytes());
            write_git_ref(&git_dir, "refs/heads/main", &commit_sha1);

            let store = ClawStore::init(root).unwrap();
            let mut importer = GitImporter::new(&store);
            let rev_id = importer
                .import_ref(&git_dir, "refs/heads/main", "heads/imported")
                .unwrap();

            let tree_id = importer.get_object_id(&tree_sha1).unwrap();
            let blob_id = importer.get_object_id(&blob_sha1).unwrap();
            (rev_id, tree_id, blob_id)
        }

        let first = import_once();
        let second = import_once();
        assert_eq!(first, second);
    }

    #[test]
    fn imports_change_refs_and_claw_change_trailer() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        let git_dir = root.join(".git");
        fs::create_dir_all(git_dir.join("objects")).unwrap();

        let blob_sha1 = write_git_object(&git_dir, "blob", b"x");
        let mut tree_body = Vec::new();
        tree_body.extend_from_slice(b"100644 x.txt\0");
        tree_body.extend_from_slice(&blob_sha1);
        let tree_sha1 = write_git_object(&git_dir, "tree", &tree_body);

        let change_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let commit_body = format!(
            "tree {}\nauthor Eve <eve@example.com> 1700000200 +0000\ncommitter Eve <eve@example.com> 1700000200 +0000\n\nImported change\n\nClaw-Change: {}\n",
            hex::encode(tree_sha1),
            change_id
        );
        let commit_sha1 = write_git_object(&git_dir, "commit", commit_body.as_bytes());
        write_git_ref(&git_dir, "refs/heads/main", &commit_sha1);
        write_git_ref(
            &git_dir,
            &format!("refs/claw/changes/{change_id}"),
            &commit_sha1,
        );

        let store = ClawStore::init(root).unwrap();
        let mut importer = GitImporter::new(&store);
        let rev_id = importer
            .import_ref(&git_dir, "refs/heads/main", "heads/imported")
            .unwrap();

        let change_ref = store.get_ref(&format!("changes/{change_id}")).unwrap();
        assert_eq!(change_ref, Some(rev_id));

        let rev = match store.load_object(&rev_id).unwrap() {
            Object::Revision(r) => r,
            other => panic!("expected revision, got {other:?}"),
        };
        assert_eq!(
            rev.change_id.map(|id| id.to_string()),
            Some(change_id.to_string())
        );
    }
}
