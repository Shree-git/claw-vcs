#![no_main]

use claw_core::cof::{
    classify_cof_version, cof_decode_with_migration, cof_encode, cof_migration_plan, cof_version,
    CofVersionSupport,
};
use claw_core::object::Object;
use claw_core::object::TypeTag;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(version) = cof_version(data) {
        let plan = cof_migration_plan(version);
        assert_eq!(plan.support(), classify_cof_version(version));
        if plan.can_write_source_version() {
            assert_eq!(plan.support(), CofVersionSupport::Native);
        }
    }

    if let Ok((tag, payload, plan)) = cof_decode_with_migration(data) {
        assert_eq!(plan.support(), CofVersionSupport::Native);
        let _ = Object::deserialize_payload(tag, &payload);
    }

    if let Some((&tag_seed, payload)) = data.split_first() {
        let tag_value = (tag_seed % 12) + 1;
        if let Some(tag) = TypeTag::from_u8(tag_value) {
            if let Ok(encoded) = cof_encode(tag, payload) {
                let (_, decoded, plan) =
                    cof_decode_with_migration(&encoded).expect("encoded COF decodes");
                assert_eq!(plan.support(), CofVersionSupport::Native);
                assert_eq!(decoded, payload);
            }
        }
    }
});
