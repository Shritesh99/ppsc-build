use alloc::collections::BTreeMap;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use parity_scale_codec::{Decode, Encode};
use prost::Message;
use std::time::Duration;

// Make imports available for the included code
#[allow(unused_imports)]
extern crate alloc;

// Generated code from ppsc-build
include!("tmp/network.protocol.rs");

// Generated code from prost-build
mod prost_generated {
    include!("tmp/network.protocol.prost.rs");
}

fn create_ppsc_transaction() -> TransactionRequest {
    TransactionRequest {
        is_priority: true,
        transaction_id: 123,
        creation_time: 456,
        memo: "test".to_string(),
        associated_ids: vec!["id1".to_string(), "id2".to_string()],
        metadata: BTreeMap::from([("key1".to_string(), 1), ("key2".to_string(), 2)]),
        sender: None,
        status: 1,
        result: None,
    }
}

fn create_prost_transaction() -> prost_generated::TransactionRequest {
    prost_generated::TransactionRequest {
        is_priority: false,
        transaction_id: 12345,
        creation_time: 1710000000,
        memo: "test_transaction".into(),
        associated_ids: Vec::new(),
        metadata: std::collections::HashMap::new(),
        sender: Some(prost_generated::Entity {
            id: "sender123".into(),
            ip_address: 0x7f000001,
        }),
        status: prost_generated::TransactionStatus::StatusPending as i32,
        result: None,
    }
}

fn bench_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("encoding");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let ppsc_tx = create_ppsc_transaction();
    let prost_tx = create_prost_transaction();

    group.bench_function("ppsc_encode", |b| b.iter(|| black_box(&ppsc_tx).encode()));

    group.bench_function("prost_encode", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            black_box(&prost_tx).encode(&mut buf).unwrap();
            buf
        })
    });

    group.finish();
}

fn bench_decoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("decoding");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let ppsc_encoded = create_ppsc_transaction().encode();
    let mut prost_encoded = Vec::new();
    create_prost_transaction()
        .encode(&mut prost_encoded)
        .unwrap();

    group.bench_function("ppsc_decode", |b| {
        b.iter(|| TransactionRequest::decode(&mut &ppsc_encoded[..]))
    });

    group.bench_function("prost_decode", |b| {
        b.iter(|| prost_generated::TransactionRequest::decode(prost_encoded.as_slice()).unwrap())
    });

    group.finish();
}

fn bench_size_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_size");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let ppsc_tx = create_ppsc_transaction();
    let prost_tx = create_prost_transaction();

    let ppsc_encoded = ppsc_tx.encode();
    let ppsc_size = ppsc_encoded.len();

    let mut prost_buf = Vec::new();
    prost_tx.encode(&mut prost_buf).unwrap();
    let prost_size = prost_buf.len();

    println!("PPSC encoded size: {} bytes", ppsc_size);
    println!("Prost encoded size: {} bytes", prost_size);

    group.finish();
}

criterion_group!(
    benches,
    bench_encoding,
    bench_decoding,
    bench_size_comparison
);
criterion_main!(benches);
