use claw_core::cof::cof_decode;
use claw_core::object::{Object, TypeTag};
use claw_core::types::Blob;
use claw_store::layout::RepoLayout;
use claw_store::loose::loose_object_path;
use claw_store::pack::{read_object_from_pack, read_pack_index, PackWriter};
use claw_store::ClawStore;
use proptest::prelude::*;

proptest! {
    #[test]
    fn store_load_blob_roundtrips(
        data in prop::collection::vec(any::<u8>(), 0..2048),
        media in prop::option::of("[a-z]+/[a-z0-9.+-]+")
    ) {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();
        let object = Object::Blob(Blob {
            data: data.clone(),
            media_type: media.clone(),
        });

        let id = store.store_object(&object).unwrap();
        prop_assert!(store.has_object(&id));
        prop_assert!(store.list_object_ids().unwrap().contains(&id));

        let raw = store.load_cof_bytes(&id).unwrap();
        let (tag, _payload) = cof_decode(&raw).unwrap();
        prop_assert_eq!(tag, TypeTag::Blob);

        match store.load_object(&id).unwrap() {
            Object::Blob(blob) => {
                prop_assert_eq!(blob.data, data);
                prop_assert_eq!(blob.media_type, media);
            }
            other => prop_assert!(false, "loaded unexpected object: {:?}", other.type_tag()),
        }
    }
}

#[test]
fn corrupt_loose_object_fails_to_load() {
    let tmp = tempfile::tempdir().unwrap();
    let store = ClawStore::init(tmp.path()).unwrap();
    let id = store
        .store_object(&Object::Blob(Blob {
            data: b"tamper target".to_vec(),
            media_type: None,
        }))
        .unwrap();
    let path = loose_object_path(store.layout(), &id);
    let mut raw = std::fs::read(&path).unwrap();
    let last = raw.len() - 1;
    raw[last] ^= 0x01;
    std::fs::write(&path, raw).unwrap();

    assert!(store.load_object(&id).is_err());
}

#[test]
fn pack_index_vectors_roundtrip_objects() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = RepoLayout::new(tmp.path());
    layout.create_dirs().unwrap();
    let mut writer = PackWriter::new();
    let first = Object::Blob(Blob {
        data: b"first".to_vec(),
        media_type: None,
    });
    let second = Object::Blob(Blob {
        data: b"second".to_vec(),
        media_type: Some("text/plain".to_string()),
    });
    let first_id = writer.add_object(&first).unwrap();
    let second_id = writer.add_object(&second).unwrap();
    let (pack_path, idx_path) = writer.write_pack(&layout).unwrap();

    let index = read_pack_index(&idx_path).unwrap();
    assert_eq!(index.len(), 2);
    assert_eq!(index[0].0, first_id);
    assert_eq!(index[1].0, second_id);

    let first_loaded = read_object_from_pack(&pack_path, index[0].1).unwrap();
    let second_loaded = read_object_from_pack(&pack_path, index[1].1).unwrap();
    assert!(matches!(first_loaded, Object::Blob(Blob { data, .. }) if data.as_slice() == b"first"));
    assert!(
        matches!(second_loaded, Object::Blob(Blob { data, .. }) if data.as_slice() == b"second")
    );
}
