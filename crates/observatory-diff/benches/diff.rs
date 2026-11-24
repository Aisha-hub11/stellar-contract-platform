//! Criterion benchmark for interface diffing on synthetic large interfaces.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use observatory_diff::diff;
use observatory_interface::ContractInterface;
use observatory_spec::parse;
use observatory_testutil::encode_spec_entries;
use observatory_testutil::spec::{spec_enum, spec_function, spec_struct, ty_udt};
use stellar_xdr::ScSpecTypeDef;

const ENTITIES: usize = 500;
const ADDED_FUNCTIONS: usize = 50;

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

fn changed_interface(old: &ContractInterface) -> ContractInterface {
    let mut new = old.clone();

    for (index, function) in new.functions.iter_mut().enumerate() {
        if index % 10 == 0 {
            let mut extra = function.inputs[0].clone();
            extra.name = format!("extra_{index}");
            function.inputs.push(extra);
        }
    }
    new.functions
        .retain(|function| !function.name.ends_with('7'));

    for index in 0..ADDED_FUNCTIONS {
        let mut function = new.functions[index].clone();
        function.name = format!("added_{index}");
        new.functions.push(function);
    }

    let mut field = new.structs[1].fields[0].clone();
    field.name = "added_field".to_string();
    new.structs[0].fields.push(field);
    new.enums.pop();
    new
}

fn bench_diff(c: &mut Criterion) {
    let old = interface();
    let new = changed_interface(&old);
    let result = diff(black_box(&old), black_box(&new));
    assert!(result.summary.total > ADDED_FUNCTIONS);
    assert!(result.summary.breaking > 0);

    c.bench_function("diff/1500_entities", |b| {
        b.iter(|| diff(black_box(&old), black_box(&new)))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = bench_diff
}
criterion_main!(benches);
