use criterion::{black_box, criterion_group, criterion_main, Criterion};

use claw_patch::binary::BinaryCodec;
use claw_patch::json_tree::JsonTreeCodec;
use claw_patch::text_line::TextLineCodec;
use claw_patch::Codec;

fn bench_patch_codecs(c: &mut Criterion) {
    let text_old = (0..2_000)
        .map(|idx| format!("line {idx}\n"))
        .collect::<String>();
    let text_new = text_old.replace("line 1500", "line 1500 changed");
    let text = TextLineCodec;
    c.bench_function("text_line_diff_2000_lines", |b| {
        b.iter(|| {
            text.diff(
                black_box(text_old.as_bytes()),
                black_box(text_new.as_bytes()),
            )
            .unwrap()
        })
    });

    let json_old = serde_json::json!({
        "items": (0..500).map(|idx| serde_json::json!({"id": idx, "enabled": true})).collect::<Vec<_>>()
    });
    let json_new = serde_json::json!({
        "items": (0..500).map(|idx| serde_json::json!({"id": idx, "enabled": idx % 3 != 0})).collect::<Vec<_>>()
    });
    let json_old = serde_json::to_vec(&json_old).unwrap();
    let json_new = serde_json::to_vec(&json_new).unwrap();
    let json = JsonTreeCodec;
    c.bench_function("json_tree_diff_500_items", |b| {
        b.iter(|| {
            json.diff(black_box(&json_old), black_box(&json_new))
                .unwrap()
        })
    });

    let binary_old = vec![0u8; 512 * 1024];
    let mut binary_new = binary_old.clone();
    binary_new[128] = 1;
    let binary = BinaryCodec;
    c.bench_function("binary_diff_512k", |b| {
        b.iter(|| {
            binary
                .diff(black_box(&binary_old), black_box(&binary_new))
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_patch_codecs);
criterion_main!(benches);
