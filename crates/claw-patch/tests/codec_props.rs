use std::collections::BTreeMap;

use claw_core::types::PatchOp;
use claw_patch::binary::BinaryCodec;
use claw_patch::json_tree::JsonTreeCodec;
use claw_patch::text_line::TextLineCodec;
use claw_patch::Codec;
use proptest::prelude::*;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct PatchVectors {
    text_line_cases: Vec<TextLineCase>,
    json_tree_cases: Vec<JsonTreeCase>,
}

#[derive(Debug, Deserialize)]
struct TextLineCase {
    name: String,
    base: String,
    target: String,
    expected_ops: Vec<ExpectedTextOp>,
}

#[derive(Debug, Deserialize)]
struct ExpectedTextOp {
    address: String,
    op_type: String,
    #[serde(default)]
    old_data_utf8: Option<String>,
    #[serde(default)]
    new_data_utf8: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JsonTreeCase {
    name: String,
    base: Value,
    left: Value,
    right: Value,
}

#[test]
fn patch_vectors_are_stable() {
    let vectors: PatchVectors =
        serde_json::from_str(include_str!("../../../tests/vectors/patch_vectors.json")).unwrap();
    let text = TextLineCodec;
    let json = JsonTreeCodec;

    for case in vectors.text_line_cases {
        let ops = text
            .diff(case.base.as_bytes(), case.target.as_bytes())
            .unwrap();
        assert_eq!(
            ops.len(),
            case.expected_ops.len(),
            "unexpected op count for {}",
            case.name
        );

        for (actual, expected) in ops.iter().zip(case.expected_ops.iter()) {
            assert_eq!(actual.address, expected.address, "{}", case.name);
            assert_eq!(actual.op_type, expected.op_type, "{}", case.name);
            assert_eq!(
                actual.old_data.as_deref(),
                expected.old_data_utf8.as_ref().map(|s| s.as_bytes()),
                "{}",
                case.name
            );
            assert_eq!(
                actual.new_data.as_deref(),
                expected.new_data_utf8.as_ref().map(|s| s.as_bytes()),
                "{}",
                case.name
            );
        }

        let applied = text.apply(case.base.as_bytes(), &ops).unwrap();
        assert_eq!(applied, case.target.as_bytes(), "{}", case.name);
        let inverted = text.invert(&ops).unwrap();
        let restored = text.apply(&applied, &inverted).unwrap();
        assert_eq!(restored, case.base.as_bytes(), "{}", case.name);
    }

    for case in vectors.json_tree_cases {
        let base = serde_json::to_vec(&case.base).unwrap();
        let left = serde_json::to_vec(&case.left).unwrap();
        let right = serde_json::to_vec(&case.right).unwrap();
        let left_ops = json.diff(&base, &left).unwrap();
        let right_ops = json.diff(&base, &right).unwrap();
        let (right_after_left, left_after_right) = json.commute(&left_ops, &right_ops).unwrap();

        let left_then_right = json
            .apply(&json.apply(&base, &left_ops).unwrap(), &right_after_left)
            .unwrap();
        let right_then_left = json
            .apply(&json.apply(&base, &right_ops).unwrap(), &left_after_right)
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&left_then_right).unwrap(),
            serde_json::from_slice::<Value>(&right_then_left).unwrap(),
            "{}",
            case.name
        );
    }
}

proptest! {
    #[test]
    fn binary_diff_apply_and_invert_roundtrip(
        base in prop::collection::vec(any::<u8>(), 0..1024),
        target in prop::collection::vec(any::<u8>(), 0..1024)
    ) {
        let codec = BinaryCodec;
        let ops = codec.diff(&base, &target).unwrap();
        let applied = codec.apply(&base, &ops).unwrap();
        prop_assert_eq!(&applied, &target);

        let inverted = codec.invert(&ops).unwrap();
        let restored = codec.apply(&target, &inverted).unwrap();
        prop_assert_eq!(restored, base);
    }

    #[test]
    fn text_line_diff_apply_roundtrip(
        base_lines in prop::collection::vec("[A-Za-z0-9_ -]{1,12}", 0..10),
        target_lines in prop::collection::vec("[A-Za-z0-9_ -]{1,12}", 0..10)
    ) {
        let codec = TextLineCodec;
        let base = text_from_lines(&base_lines);
        let target = text_from_lines(&target_lines);
        let ops = codec.diff(&base, &target).unwrap();
        let applied = codec.apply(&base, &ops).unwrap();
        prop_assert_eq!(&applied, &target);
    }

    #[test]
    fn text_line_single_replace_invert_roundtrip(
        base_line in "[A-Za-z0-9_ -]{1,12}",
        target_line in "[A-Za-z0-9_ -]{1,12}"
    ) {
        let codec = TextLineCodec;
        let base_lines = vec![base_line];
        let target_lines = vec![target_line];
        let base = text_from_lines(&base_lines);
        let target = text_from_lines(&target_lines);
        let ops = codec.diff(&base, &target).unwrap();
        let applied = codec.apply(&base, &ops).unwrap();
        prop_assert_eq!(&applied, &target);
        let inverted = codec.invert(&ops).unwrap();
        let restored = codec.apply(&target, &inverted).unwrap();
        prop_assert_eq!(restored, base);
    }

    #[test]
    fn text_line_non_overlapping_insert_ops_commute(
        base_lines in prop::collection::vec("[A-Za-z0-9_ -]{1,12}", 2..10),
        left_line in "[A-Za-z0-9_ -]{1,12}",
        right_line in "[A-Za-z0-9_ -]{1,12}",
        left_index in 0usize..8,
        gap in 1usize..8
    ) {
        let codec = TextLineCodec;
        let base = text_from_lines(&base_lines);
        let right_index = left_index + gap;
        prop_assume!(left_index < base_lines.len());
        prop_assume!(right_index <= base_lines.len());

        let left = vec![insert_op(left_index, &left_line)];
        let right = vec![insert_op(right_index, &right_line)];
        let (right_after_left, left_after_right) = codec.commute(&left, &right).unwrap();

        let left_then_right = codec.apply(&codec.apply(&base, &left).unwrap(), &right_after_left).unwrap();
        let right_then_left = codec.apply(&codec.apply(&base, &right).unwrap(), &left_after_right).unwrap();
        prop_assert_eq!(left_then_right, right_then_left);
    }

    #[test]
    fn json_tree_flat_objects_apply_and_invert_roundtrip(
        base_map in prop::collection::btree_map("[a-z][a-z0-9_]{0,6}", -1000i64..1000, 0..8),
        target_map in prop::collection::btree_map("[a-z][a-z0-9_]{0,6}", -1000i64..1000, 0..8)
    ) {
        let codec = JsonTreeCodec;
        let base = json_object_bytes(base_map);
        let target = json_object_bytes(target_map);
        let ops = codec.diff(&base, &target).unwrap();
        let applied = codec.apply(&base, &ops).unwrap();
        prop_assert_eq!(
            serde_json::from_slice::<Value>(&applied).unwrap(),
            serde_json::from_slice::<Value>(&target).unwrap()
        );

        let inverted = codec.invert(&ops).unwrap();
        let restored = codec.apply(&applied, &inverted).unwrap();
        prop_assert_eq!(
            serde_json::from_slice::<Value>(&restored).unwrap(),
            serde_json::from_slice::<Value>(&base).unwrap()
        );
    }
}

fn text_from_lines(lines: &[String]) -> Vec<u8> {
    if lines.is_empty() {
        Vec::new()
    } else {
        format!("{}\n", lines.join("\n")).into_bytes()
    }
}

fn insert_op(index: usize, line: &str) -> PatchOp {
    PatchOp {
        address: format!("L{index}"),
        op_type: "insert".to_string(),
        old_data: None,
        new_data: Some(format!("{line}\n").into_bytes()),
        context_hash: None,
    }
}

fn json_object_bytes(map: BTreeMap<String, i64>) -> Vec<u8> {
    serde_json::to_vec(&map).unwrap()
}
