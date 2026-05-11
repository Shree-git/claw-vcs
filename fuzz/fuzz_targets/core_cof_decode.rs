#![no_main]

use claw_core::cof::{cof_decode, cof_encode, cof_peek_type_tag};
use claw_core::object::TypeTag;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = cof_peek_type_tag(data);
    let _ = cof_decode(data);

    if let Some((&tag_seed, payload)) = data.split_first() {
        let tag_value = (tag_seed % 12) + 1;
        if let Some(tag) = TypeTag::from_u8(tag_value) {
            if let Ok(encoded) = cof_encode(tag, payload) {
                let _ = cof_peek_type_tag(&encoded);
                let _ = cof_decode(&encoded);
            }
        }
    }
});
