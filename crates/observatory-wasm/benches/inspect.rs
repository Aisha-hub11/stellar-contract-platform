//! Criterion benchmark for WASM inspection on a synthetic large module.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use observatory_wasm::inspect;

/// Number of exported functions; each has a one-instruction body.
const FUNCTIONS: usize = 2_000;

fn large_module() -> Vec<u8> {
    let mut wat = String::from("(module\n");
    for index in 0..FUNCTIONS {
        wat.push_str(&format!(
            "  (func $f{index} (export \"f{index}\") (result i32) i32.const {index})\n"
        ));
    }
    wat.push_str(")\n");
    wat::parse_str(&wat).expect("generated module must be valid WAT")
}

fn bench_inspect(c: &mut Criterion) {
    let bytes = large_module();
    let report = inspect(black_box(&bytes)).expect("generated module must inspect");
    assert_eq!(report.function_count, FUNCTIONS);
    assert_eq!(report.exports.len(), FUNCTIONS);

    c.bench_function("wasm/inspect/2000_functions", |b| {
        b.iter(|| inspect(black_box(&bytes)).expect("inspect"))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = bench_inspect
}
criterion_main!(benches);
