#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(not(feature = "std"))]
extern crate alloc;
use libc_print::std_name::{dbg, eprintln, println};

#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap;
#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::time::Duration;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use parity_scale_codec::{Decode, Encode};

// Generated code from ppsc-build
include!("tmp/network.protocol.rs");

fn create_ppsc_transaction() -> TransactionRequest {
    TransactionRequest {
        is_priority: false,
        transaction_id: 12345,
        creation_time: 1710000000,
        memo: String::from("test_transaction"),
        associated_ids: Vec::new(),
        metadata: BTreeMap::new(),
        sender: Some(Entity {
            id: String::from("sender123"),
            ip_address: 0x7f000001,
        }),
        status: TransactionStatus::StatusPending as i32,
        result: None,
    }
}

fn bench_ppsc_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("ppsc_encoding");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let ppsc_tx = create_ppsc_transaction();

    group.bench_function("ppsc_encode", |b| b.iter(|| black_box(&ppsc_tx).encode()));

    group.finish();
}

fn bench_ppsc_decoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("ppsc_decoding");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let ppsc_encoded = create_ppsc_transaction().encode();

    group.bench_function("ppsc_decode", |b| {
        b.iter(|| TransactionRequest::decode(&mut &ppsc_encoded[..]))
    });

    group.finish();
}

fn bench_ppsc_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("ppsc_message_size");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let ppsc_tx = create_ppsc_transaction();
    let ppsc_encoded = ppsc_tx.encode();
    let ppsc_size = ppsc_encoded.len();
    println!("PPSC encoded size: {} bytes", ppsc_size);

    group.finish();
}

criterion_group!(
    ppsc_benches,
    bench_ppsc_encoding,
    bench_ppsc_decoding,
    bench_ppsc_size
);
criterion_main!(ppsc_benches);
