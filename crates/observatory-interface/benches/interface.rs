//! Criterion benchmarks for interface canonicalization and fingerprinting.

use std::time::Duration;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use observatory_interface::ContractInterface;
use observatory_spec::parse;
use observatory_testutil::encode_spec_entries;
use observatory_testutil::spec::{spec_enum, spec_function, spec_struct, ty_udt};
use stellar_xdr::ScSpecTypeDef;

const ENTITIES: usize = 500;

fn interface() -> ContractInterface {
    let mut entries = Vec::with_capacity(ENTITIES * 3);
    for index in 0..ENTITIES {
        let next = format!("Struct{}", (index + 1) % ENTITIES);
        entries.push(spec_struct(
            &format!("Struct{index}"),
            &[
                ("a", ScSpecTypeDef::U32),
                ("b", ScSpecTypeDef::I128),
                ("next", ty_udt(&next)),
            ],
        ));
        entries.push(spec_function(
            &format!("fn_{index}"),
            &[
                ("amount", ScSpecTypeDef::I128),
                ("to", ScSpecTypeDef::Address),
            ],
            Some(ScSpecTypeDef::U64),
        ));
        entries.push(spec_enum(
            &format!("Enum{index}"),
            &[("A", 0), ("B", 1), ("C", 2)],
        ));
    }
    let spec = parse(&encode_spec_entries(&entries)).expect("generated spec must parse");
    ContractInterface::from_spec(spec)
}

fn bench_interface(c: &mut Criterion) {
    let canonical = interface();
    assert_eq!(canonical.functions.len(), ENTITIES);
    assert_eq!(canonical.structs.len(), ENTITIES);
    assert_eq!(canonical.enums.len(), ENTITIES);

    let mut unsorted = canonical.clone();
    unsorted.functions.reverse();
    unsorted.structs.reverse();
    unsorted.enums.reverse();

    c.bench_function("interface/canonicalize/1500_entities", |b| {
        b.iter_batched(
            || unsorted.clone(),
            |mut candidate| candidate.canonicalize(),
            BatchSize::SmallInput,
        );
    });

    let json = canonical.canonical_json().expect("canonical JSON");
    assert!(!json.is_empty());
    c.bench_function("interface/fingerprint/1500_entities", |b| {
        b.iter(|| canonical.fingerprint().expect("fingerprint"))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = bench_interface
}
criterion_main!(benches);
