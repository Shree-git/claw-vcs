#![no_main]

use claw_core::object::Object;
use claw_core::types::Blob;
use claw_store::pack::{read_object_from_pack, read_pack_index, PackWriter};
use claw_store::ClawStore;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let capped = &data[..data.len().min(4096)];
    if let Ok(tmp) = tempfile::tempdir() {
        if let Ok(store) = ClawStore::init(tmp.path()) {
            let object = Object::Blob(Blob {
                data: capped.to_vec(),
                media_type: None,
            });
            if let Ok(id) = store.store_object(&object) {
                let _ = store.has_object(&id);
                let _ = store.load_cof_bytes(&id);
                let _ = store.load_object(&id);
                let _ = store.list_object_ids();
            }

            let mut pack = PackWriter::new();
            if pack.add_object(&object).is_ok() {
                if let Ok((pack_path, index_path)) = pack.write_pack(store.layout()) {
                    if let Ok(index) = read_pack_index(&index_path) {
                        for (_, offset) in index {
                            let _ = read_object_from_pack(&pack_path, offset);
                        }
                    }
                }
            }
        }
    }
});
