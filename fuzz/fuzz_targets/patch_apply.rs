#![no_main]

use claw_patch::binary::BinaryCodec;
use claw_patch::json_tree::JsonTreeCodec;
use claw_patch::text_line::TextLineCodec;
use claw_patch::Codec;
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    let split = data.len() / 2;
    let (base, target) = data.split_at(split);

    let binary = BinaryCodec;
    if let Ok(ops) = binary.diff(base, target) {
        let _ = binary.apply(base, &ops);
        if let Ok(inverted) = binary.invert(&ops) {
            let _ = binary.apply(target, &inverted);
        }
    }

    if std::str::from_utf8(base).is_ok() && std::str::from_utf8(target).is_ok() {
        let text = TextLineCodec;
        if let Ok(ops) = text.diff(base, target) {
            let _ = text.apply(base, &ops);
            if let Ok(inverted) = text.invert(&ops) {
                let _ = text.apply(target, &inverted);
            }
        }
    }

    if let (Ok(base_json), Ok(target_json)) = (
        serde_json::from_slice::<Value>(base),
        serde_json::from_slice::<Value>(target),
    ) {
        let base = serde_json::to_vec(&base_json).unwrap_or_default();
        let target = serde_json::to_vec(&target_json).unwrap_or_default();
        let json = JsonTreeCodec;
        if let Ok(ops) = json.diff(&base, &target) {
            let _ = json.apply(&base, &ops);
            if let Ok(inverted) = json.invert(&ops) {
                let _ = json.apply(&target, &inverted);
            }
        }
    }
});
