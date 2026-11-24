//! Criterion benchmark for specification parsing on a synthetic large section.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use observatory_spec::parse;
use observatory_testutil::encode_spec_entries;
use observatory_testutil::spec::{spec_enum, spec_function, spec_struct, ty_udt};
use stellar_xdr::ScSpecTypeDef;

/// Number of each entity kind; the section therefore holds 3000 entries.
const ENTITIES: usize = 1_000;

fn specification() -> Vec<u8> {
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
    encode_spec_entries(&entries)
}

fn bench_parse(c: &mut Criterion) {
    let bytes = specification();
    let parsed = parse(black_box(&bytes)).expect("generated spec must parse");
    assert_eq!(parsed.structs.len(), ENTITIES);
    assert_eq!(parsed.functions.len(), ENTITIES);
    assert_eq!(parsed.enums.len(), ENTITIES);

    c.bench_function("spec/parse/3000_entries", |b| {
        b.iter(|| parse(black_box(&bytes)).expect("parse"))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = bench_parse
}
criterion_main!(benches);
