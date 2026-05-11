use claw_core::object::TypeTag;
use claw_core::types::{validate_tree_entry_name, FileMode, Tree, TreeEntry};
use claw_core::{content_hash, ObjectId};
use claw_store::refs::{list_refs, validate_ref_name, write_ref};
use claw_store::{ClawStore, StoreError};

#[test]
fn refs_reject_windows_separators_before_touching_disk() {
    let tmp = tempfile::tempdir().expect("temp repo");
    let store = ClawStore::init(tmp.path()).expect("init store");
    let id = content_hash(TypeTag::Blob, b"path safety");

    for name in [
        r"heads\main",
        r"heads/main\feature",
        r"C:\repo\.claw\refs\heads\main",
        r"\server\share\refs\main",
    ] {
        let err = validate_ref_name(name).expect_err("ref name should be rejected");
        assert!(matches!(err, StoreError::InvalidRefName(_)));

        let err = write_ref(store.layout(), name, &id).expect_err("write_ref should reject name");
        assert!(matches!(err, StoreError::InvalidRefName(_)));
    }

    assert!(
        !store.layout().refs_dir().join(r"heads\main").exists(),
        "backslash ref names must not be materialized as literal files"
    );
}

#[test]
fn ref_prefix_listing_rejects_windows_style_traversal_inputs() {
    let tmp = tempfile::tempdir().expect("temp repo");
    let store = ClawStore::init(tmp.path()).expect("init store");

    for prefix in [r"heads\", r"..\outside", r"C:\repo\refs"] {
        let err = list_refs(store.layout(), prefix).expect_err("prefix should be rejected");
        assert!(matches!(err, StoreError::InvalidRefName(_)));
    }

    let id = content_hash(TypeTag::Blob, b"main");
    write_ref(store.layout(), "heads/main", &id).expect("write normal ref");
    assert_eq!(list_refs(store.layout(), "heads").unwrap().len(), 1);
}

#[test]
fn tree_entries_reject_windows_path_like_names() {
    for name in [
        r"dir\file.txt",
        r"C:\Users\project\file.txt",
        "C:/Users/project/file.txt",
        r"..\outside",
    ] {
        assert!(
            validate_tree_entry_name(name).is_err(),
            "tree entry name should be rejected: {name}"
        );
    }

    let tree = Tree {
        entries: vec![TreeEntry {
            name: r"nested\file.txt".to_string(),
            mode: FileMode::Regular,
            object_id: ObjectId::from_bytes([0x11; 32]),
        }],
    };
    assert!(tree.validate().is_err());
}

#[cfg(windows)]
#[test]
fn windows_absolute_ref_paths_are_rejected_by_platform_prefix_parser() {
    let err = validate_ref_name("C:/repo/.claw/refs/heads/main")
        .expect_err("windows absolute paths should not be valid refs");
    assert!(matches!(err, StoreError::InvalidRefName(_)));
}
