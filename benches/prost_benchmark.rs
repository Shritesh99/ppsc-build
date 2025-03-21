use criterion::{Criterion, black_box, criterion_group, criterion_main};
use prost::Message;
use std::collections::HashMap;
use std::time::Duration;

// Generated code from prost-build
mod prost_generated {
    include!("tmp/network.protocol.prost.rs");
}

fn create_prost_transaction() -> prost_generated::TransactionRequest {
    prost_generated::TransactionRequest {
        is_priority: false,
        transaction_id: 12345,
        creation_time: 1710000000,
        memo: "test_transaction".into(),
        associated_ids: Vec::new(),
        metadata: HashMap::new(),
        sender: Some(prost_generated::Entity {
            id: "sender123".into(),
            ip_address: 0x7f000001,
        }),
        status: prost_generated::TransactionStatus::StatusPending as i32,
        result: None,
    }
}

fn bench_prost_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("prost_encoding");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let prost_tx = create_prost_transaction();

    group.bench_function("prost_encode", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            black_box(&prost_tx).encode(&mut buf).unwrap();
            buf
        })
    });

    group.finish();
}

fn bench_prost_decoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("prost_decoding");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let mut prost_encoded = Vec::new();
    create_prost_transaction()
        .encode(&mut prost_encoded)
        .unwrap();

    group.bench_function("prost_decode", |b| {
        b.iter(|| prost_generated::TransactionRequest::decode(prost_encoded.as_slice()).unwrap())
    });

    group.finish();
}

fn bench_prost_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("prost_message_size");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let prost_tx = create_prost_transaction();
    let mut prost_buf = Vec::new();
    prost_tx.encode(&mut prost_buf).unwrap();
    let prost_size = prost_buf.len();
    println!("Prost encoded size: {} bytes", prost_size);

    group.finish();
}

criterion_group!(
    prost_benches,
    bench_prost_encoding,
    bench_prost_decoding,
    bench_prost_size
);
criterion_main!(prost_benches);
