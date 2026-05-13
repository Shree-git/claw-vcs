use criterion::{black_box, criterion_group, criterion_main, Criterion};

use claw_core::cof::{cof_decode, cof_encode};
use claw_core::hash::content_hash;
use claw_core::object::TypeTag;

fn bench_cof(c: &mut Criterion) {
    let payload = vec![42u8; 64 * 1024];

    c.bench_function("cof_encode_blob_64k", |b| {
        b.iter(|| cof_encode(TypeTag::Blob, black_box(&payload)).unwrap())
    });

    let encoded = cof_encode(TypeTag::Blob, &payload).unwrap();
    c.bench_function("cof_decode_blob_64k", |b| {
        b.iter(|| cof_decode(black_box(&encoded)).unwrap())
    });

    c.bench_function("content_hash_blob_64k", |b| {
        b.iter(|| content_hash(TypeTag::Blob, black_box(&payload)))
    });
}

criterion_group!(benches, bench_cof);
criterion_main!(benches);
