#![no_main]

use claw_patch::json_tree::JsonTreeCodec;
use claw_patch::Codec;
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 {
        return;
    }

    let first = data.len() / 3;
    let second = first * 2;
    let (base, rest) = data.split_at(first);
    let (left, right) = rest.split_at(second.saturating_sub(first));

    let (Ok(base_json), Ok(left_json), Ok(right_json)) = (
        serde_json::from_slice::<Value>(base),
        serde_json::from_slice::<Value>(left),
        serde_json::from_slice::<Value>(right),
    ) else {
        return;
    };

    let base = serde_json::to_vec(&base_json).unwrap_or_default();
    let left = serde_json::to_vec(&left_json).unwrap_or_default();
    let right = serde_json::to_vec(&right_json).unwrap_or_default();

    let codec = JsonTreeCodec;
    let _ = codec.merge3(&base, &left, &right);

    if let (Ok(left_ops), Ok(right_ops)) = (codec.diff(&base, &left), codec.diff(&base, &right)) {
        let _ = codec.commute(&left_ops, &right_ops);
    }
});
