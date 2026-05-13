#![no_main]

use claw_core::id::{ChangeId, IntentId, ObjectId};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = ObjectId::from_display(text);
        let _ = ObjectId::from_hex(text);
        let _ = IntentId::from_string(text);
        let _ = ChangeId::from_string(text);
    }

    if data.len() == 32 {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(data);
        let id = ObjectId::from_bytes(bytes);
        let _ = ObjectId::from_display(&id.to_string());
        let _ = ObjectId::from_hex(&id.to_hex());
    }
});
